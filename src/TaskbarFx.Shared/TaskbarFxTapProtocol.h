#pragma once

// Shared host ↔ TAP protocol. Lives in Suite (TaskbarFx.Native) and in explorer
// (TaskbarFx.Tap). Keep this header free of WinRT.

#include <stdint.h>
#include <guiddef.h>

// {B7C91E2A-4D53-4F80-9A1C-6E8F2B4D7C10}
static const CLSID CLSID_TaskbarFxTapSite =
{ 0xb7c91e2a, 0x4d53, 0x4f80, { 0x9a, 0x1c, 0x6e, 0x8f, 0x2b, 0x4d, 0x7c, 0x10 } };

static const wchar_t kTaskbarFxTapShmName[] = L"Local\\Suite.TaskbarFx.Tap.Shm";
static const wchar_t kTaskbarFxTapReadyEventName[] = L"Local\\Suite.TaskbarFx.Tap.Ready";
static const wchar_t kTaskbarFxTapApplyEventName[] = L"Local\\Suite.TaskbarFx.Tap.Apply";
static const wchar_t kTaskbarFxTapStopEventName[] = L"Local\\Suite.TaskbarFx.Tap.Stop";

static const uint32_t kTaskbarFxTapMagic = 0x31584654u; // TFX1
static const uint32_t kTaskbarFxTapVersion = 1;

enum TaskbarFxTapMode
{
    TaskbarFxTapModeNormal = 0,
    TaskbarFxTapModeOpaque = 1,
    TaskbarFxTapModeClear = 2,
    TaskbarFxTapModeAcrylic = 3
};

enum TaskbarFxTapResultCode
{
    TaskbarFxTapResultOk = 0,
    TaskbarFxTapResultPendingFills = 1,
    TaskbarFxTapResultAcrylicFallback = 2,
    TaskbarFxTapResultFailed = 3
};

#pragma pack(push, 8)
struct TaskbarFxTapShm
{
    uint32_t magic;
    uint32_t version;
    volatile uint32_t seq;
    volatile uint32_t ack;
    volatile int32_t mode;
    volatile uint32_t argb;
    volatile int32_t result;
    volatile int32_t ready;
    volatile uint32_t fillCount;
    volatile uint32_t hostPid;
    volatile uint32_t explorerPid;
};
#pragma pack(pop)
