use std::collections::HashMap;

use crate::types::InterfaceSnapshot;

pub struct AdapterResolver;

impl AdapterResolver {
    pub fn resolve<'a>(
        table: &'a [InterfaceSnapshot],
        preferred_if_index: Option<u32>,
        preferred_alias: Option<&str>,
        previous_table: Option<&[InterfaceSnapshot]>,
    ) -> Option<&'a InterfaceSnapshot> {
        let mut preferred: Option<&InterfaceSnapshot> = None;

        if let Some(idx) = preferred_if_index
            && let Some(by_index) = table.iter().find(|x| x.if_index == idx)
            && Self::is_usable_preferred(by_index)
        {
            preferred = Some(by_index);
        }

        if preferred.is_none()
            && let Some(alias) = preferred_alias.map(str::trim).filter(|s| !s.is_empty())
            && let Some(by_alias) = table.iter().find(|x| x.alias.eq_ignore_ascii_case(alias))
            && Self::is_usable_preferred(by_alias)
        {
            preferred = Some(by_alias);
        }

        if let Some(prev_table) = previous_table
            && let Some(active) = Self::pick_active(table, prev_table)
        {
            let preferred_idle = match preferred {
                None => true,
                Some(p) => Self::traffic_delta_slice(p, prev_table) == 0,
            };
            if preferred_idle {
                return Some(active);
            }
        }

        if let Some(p) = preferred {
            return Some(p);
        }

        Self::pick_default(table)
    }

    fn pick_active<'a>(
        table: &'a [InterfaceSnapshot],
        previous_table: &[InterfaceSnapshot],
    ) -> Option<&'a InterfaceSnapshot> {
        let mut previous: HashMap<u32, &InterfaceSnapshot> = HashMap::with_capacity(previous_table.len());
        for row in previous_table {
            previous.insert(row.if_index, row);
        }

        table
            .iter()
            .filter(|x| Self::is_usable_preferred(x))
            .filter_map(|x| {
                let delta = Self::traffic_delta_map(x, &previous);
                (delta > 0).then_some((x, delta))
            })
            .max_by(|(a, da), (b, db)| {
                da.cmp(db)
                    .then_with(|| a.media_connected.cmp(&b.media_connected))
                    .then_with(|| b.if_index.cmp(&a.if_index))
            })
            .map(|(x, _)| x)
    }

    fn traffic_delta_slice(current: &InterfaceSnapshot, previous_table: &[InterfaceSnapshot]) -> u64 {
        let Some(old) = previous_table.iter().find(|x| x.if_index == current.if_index) else {
            return 0;
        };
        if current.in_octets < old.in_octets || current.out_octets < old.out_octets {
            return 0;
        }
        (current.in_octets - old.in_octets) + (current.out_octets - old.out_octets)
    }

    fn traffic_delta_map(
        current: &InterfaceSnapshot,
        previous: &HashMap<u32, &InterfaceSnapshot>,
    ) -> u64 {
        let Some(old) = previous.get(&current.if_index) else {
            return 0;
        };
        if current.in_octets < old.in_octets || current.out_octets < old.out_octets {
            return 0;
        }
        (current.in_octets - old.in_octets) + (current.out_octets - old.out_octets)
    }

    pub fn is_usable_preferred(x: &InterfaceSnapshot) -> bool {
        x.is_up && !x.is_loopback() && !x.is_tunnel()
    }

    pub fn pick_default(table: &[InterfaceSnapshot]) -> Option<&InterfaceSnapshot> {
        fn score(x: &InterfaceSnapshot) -> i32 {
            let mut s = 0;
            if x.is_up {
                s += 100;
            }
            if x.media_connected {
                s += 50;
            }
            if !AdapterResolver::looks_virtual(x) {
                s += 40;
            }
            if x.if_type == 6 {
                s += 20;
            } else if x.if_type == 71 {
                s += 15;
            }
            if !x.is_loopback() && !x.is_tunnel() {
                s += 5;
            }
            let octets = x.in_octets.saturating_add(x.out_octets);
            if octets > 1_000_000 {
                s += 10;
            } else if octets > 10_000 {
                s += 4;
            }
            s
        }

        table
            .iter()
            .filter(|x| !x.is_loopback() && !x.is_tunnel())
            .max_by(|a, b| {
                score(a)
                    .cmp(&score(b))
                    .then_with(|| {
                        a.in_octets
                            .saturating_add(a.out_octets)
                            .cmp(&b.in_octets.saturating_add(b.out_octets))
                    })
                    .then_with(|| b.if_index.cmp(&a.if_index))
            })
    }

    pub fn looks_virtual(x: &InterfaceSnapshot) -> bool {
        let hay = format!("{} {}", x.description, x.alias).to_ascii_lowercase();
        [
            "vmware",
            "virtualbox",
            "hyper-v",
            "wi-fi direct",
            "bluetooth",
            "loopback",
            "virtual adapter",
            "vethernet",
        ]
        .iter()
        .any(|needle| hay.contains(needle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(idx: u32, alias: &str, desc: &str, if_type: u32, up: bool, in_b: u64, out_b: u64) -> InterfaceSnapshot {
        InterfaceSnapshot {
            if_index: idx,
            alias: alias.into(),
            description: desc.into(),
            is_up: up,
            if_type,
            media_connected: up,
            in_octets: in_b,
            out_octets: out_b,
        }
    }

    #[test]
    fn ignores_loopback_and_prefers_physical_over_vmware() {
        let table = vec![
            mk(1, "Loopback", "Software Loopback", 24, true, 99_000_000, 0),
            mk(2, "VMware Network Adapter", "VMware Virtual Ethernet", 6, true, 500_000, 100),
            mk(9, "WLAN", "Intel Wi-Fi 6", 71, true, 5_000_000, 200_000),
        ];
        let picked = AdapterResolver::resolve(&table, None, None, None).unwrap();
        assert_eq!(picked.if_index, 9);
    }

    #[test]
    fn switches_to_active_adapter_when_preferred_is_idle() {
        let prev = vec![
            mk(3, "Ethernet", "Realtek PCIe", 6, true, 1000, 1000),
            mk(9, "WLAN", "Intel Wi-Fi 6", 71, true, 2000, 2000),
        ];
        let curr = vec![
            mk(3, "Ethernet", "Realtek PCIe", 6, true, 1000, 1000),
            mk(9, "WLAN", "Intel Wi-Fi 6", 71, true, 52_000, 12_000),
        ];
        let picked = AdapterResolver::resolve(&curr, Some(3), Some("Ethernet"), Some(&prev)).unwrap();
        assert_eq!(picked.if_index, 9);
    }
}
