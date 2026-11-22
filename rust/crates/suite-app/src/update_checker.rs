use serde::Deserialize;
use suite_contracts::UpdateSettings;
use suite_platform::constants::to_wide_null;
use windows::Win32::Networking::WinHttp::{
    WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, WINHTTP_FLAG_SECURE, WinHttpCloseHandle, WinHttpConnect,
    WinHttpOpen, WinHttpOpenRequest, WinHttpQueryDataAvailable, WinHttpReadData,
    WinHttpReceiveResponse, WinHttpSendRequest,
};
use windows::core::PCWSTR;

use crate::messages::UpdateCheckResult;

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: Option<String>,
    html_url: Option<String>,
    prerelease: Option<bool>,
    draft: Option<bool>,
}

pub struct UpdateChecker;

impl UpdateChecker {
    pub const LOCAL_VERSION: &'static str = env!("CARGO_PKG_VERSION");
    pub const CHECKING: &'static str = "正在检查更新…";
    pub const UP_TO_DATE: &'static str = "已是最新版本。";
    pub const FOUND: &'static str = "发现新版本。";
    pub const FAIL_NETWORK: &'static str = "检查更新失败，请稍后再试。";
    pub const FAIL_EMPTY: &'static str = "这个通道暂时没有可用更新。";
    pub const FAIL_NOT_WIRED: &'static str = "更新检查尚未在本机启用。";

    pub fn check_blocking(settings: &UpdateSettings) -> UpdateCheckResult {
        let mut cfg = settings.clone();
        cfg.normalize();
        if cfg.owner.is_empty() || cfg.repo.is_empty() {
            return Self::fail(Self::FAIL_NOT_WIRED);
        }

        let path = format!("/repos/{}/{}/releases?per_page=10", cfg.owner, cfg.repo);
        let body = match Self::https_get("api.github.com", &path) {
            Ok(b) => b,
            Err(_) => return Self::fail(Self::FAIL_NETWORK),
        };

        let Ok(releases) = serde_json::from_slice::<Vec<GitHubRelease>>(&body) else {
            return Self::fail(Self::FAIL_NETWORK);
        };
        if releases.is_empty() {
            return Self::fail(Self::FAIL_EMPTY);
        }

        let want_pre = cfg.channel.eq_ignore_ascii_case(UpdateSettings::CHANNEL_PREVIEW);
        let mut matched = releases.iter().find(|r| {
            let draft = r.draft.unwrap_or(false);
            let pre = r.prerelease.unwrap_or(false);
            !draft && (if want_pre { pre } else { !pre })
        });
        if matched.is_none() && want_pre {
            matched = releases.iter().find(|r| !r.draft.unwrap_or(false));
        }

        let Some(rel) = matched else {
            return Self::fail(Self::FAIL_EMPTY);
        };
        let tag = rel.tag_name.as_deref().unwrap_or("");
        let html = rel.html_url.as_deref().unwrap_or("");
        let remote = Self::normalize_version(tag);
        let local = Self::normalize_version(Self::LOCAL_VERSION);
        let newer = Self::compare_semver(&remote, &local) > 0;
        let channel_zh = cfg.channel_display_zh().to_string();

        if !newer {
            UpdateCheckResult {
                ok: true,
                message: format!("{}（当前 {local}，远端 {tag}）", Self::UP_TO_DATE),
                version: Some(tag.to_string()),
                html_url: Some(html.to_string()),
                channel_zh: Some(channel_zh),
                has_update: false,
            }
        } else {
            UpdateCheckResult {
                ok: true,
                message: format!("{}（当前 {local} → {channel_zh} {tag}）", Self::FOUND),
                version: Some(tag.to_string()),
                html_url: Some(html.to_string()),
                channel_zh: Some(channel_zh),
                has_update: true,
            }
        }
    }

    fn fail(msg: &str) -> UpdateCheckResult {
        UpdateCheckResult {
            ok: false,
            message: msg.to_string(),
            version: None,
            html_url: None,
            channel_zh: None,
            has_update: false,
        }
    }

    fn https_get(host: &str, path: &str) -> Result<Vec<u8>, ()> {
        let ua = to_wide_null(&format!("Suite/{}", Self::LOCAL_VERSION));
        let whost = to_wide_null(host);
        let wverb = to_wide_null("GET");
        let wpath = to_wide_null(path);
        let hdrs = to_wide_null("Accept: application/vnd.github+json\r\n");

        unsafe {
            let session = WinHttpOpen(
                PCWSTR(ua.as_ptr()),
                WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            );
            if session.is_null() {
                return Err(());
            }
            let conn = WinHttpConnect(session, PCWSTR(whost.as_ptr()), 443, 0);
            if conn.is_null() {
                let _ = WinHttpCloseHandle(session);
                return Err(());
            }
            let req = WinHttpOpenRequest(
                conn,
                PCWSTR(wverb.as_ptr()),
                PCWSTR(wpath.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                WINHTTP_FLAG_SECURE,
            );
            if req.is_null() {
                let _ = WinHttpCloseHandle(conn);
                let _ = WinHttpCloseHandle(session);
                return Err(());
            }

            let ok = WinHttpSendRequest(
                req,
                Some(&hdrs[..hdrs.len() - 1]),
                None,
                0,
                0,
                0,
            )
            .is_ok()
                && WinHttpReceiveResponse(req, std::ptr::null_mut()).is_ok();

            let mut body = Vec::new();
            if ok {
                loop {
                    let mut avail = 0u32;
                    if WinHttpQueryDataAvailable(req, &mut avail).is_err() || avail == 0 {
                        break;
                    }
                    let mut chunk = vec![0u8; avail as usize];
                    let mut read = 0u32;
                    if WinHttpReadData(req, chunk.as_mut_ptr().cast(), avail, &mut read).is_err()
                        || read == 0
                    {
                        break;
                    }
                    body.extend_from_slice(&chunk[..read as usize]);
                }
            }

            let _ = WinHttpCloseHandle(req);
            let _ = WinHttpCloseHandle(conn);
            let _ = WinHttpCloseHandle(session);

            if ok { Ok(body) } else { Err(()) }
        }
    }

    pub fn normalize_version(raw: &str) -> String {
        let mut s = raw.trim();
        if let Some(rest) = s.strip_prefix('v').or_else(|| s.strip_prefix('V')) {
            s = rest;
        }
        if let Some(pos) = s.find(['-', '+']) {
            s = &s[..pos];
        }
        if s.is_empty() {
            "0.0.0".to_string()
        } else {
            s.to_string()
        }
    }

    pub fn compare_semver(a: &str, b: &str) -> i32 {
        let pa = Self::parse_parts(a);
        let pb = Self::parse_parts(b);
        for i in 0..3 {
            if pa[i] > pb[i] {
                return 1;
            }
            if pa[i] < pb[i] {
                return -1;
            }
        }
        0
    }

    fn parse_parts(v: &str) -> [i32; 3] {
        let mut parts = [0i32; 3];
        for (i, bit) in v.split('.').filter(|s| !s.is_empty()).take(3).enumerate() {
            parts[i] = bit.parse::<i32>().unwrap_or(0);
        }
        parts
    }
}
