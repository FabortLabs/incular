// Opt-in Windows control for the WGPU DX12 presentation-memory investigation.
// This is a diagnostic program, not an Incular backend or benchmark app.
// Set INCULAR_GPU_PROBE_STAGE to window, device, configure, or present and
// INCULAR_BENCH_READY to a marker path; probe-gpu-stages.py samples the process.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <d3d12.h>
#include <dxgi1_6.h>
#include <wrl/client.h>
#include <cstdio>
#include <cstdlib>
#include <cwchar>

using Microsoft::WRL::ComPtr;

static void require(HRESULT result, const char* operation) {
    if (FAILED(result)) {
        std::fprintf(stderr, "%s failed: 0x%08lx\n", operation,
                     static_cast<unsigned long>(result));
        std::exit(1);
    }
}

static LRESULT CALLBACK window_proc(HWND window, UINT message, WPARAM wparam,
                                     LPARAM lparam) {
    if (message == WM_DESTROY) {
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(window, message, wparam, lparam);
}

static void ready_and_idle() {
    wchar_t path[32768];
    DWORD length = GetEnvironmentVariableW(L"INCULAR_BENCH_READY", path, 32768);
    if (length == 0 || length >= 32768) {
        std::fputs("INCULAR_BENCH_READY is required\n", stderr);
        std::exit(1);
    }
    HANDLE marker = CreateFileW(path, GENERIC_WRITE, FILE_SHARE_READ, nullptr,
                                CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (marker == INVALID_HANDLE_VALUE) {
        std::fputs("failed to create readiness marker\n", stderr);
        std::exit(1);
    }
    const char message[] = "native D3D12 probe ready\n";
    DWORD written = 0;
    if (!WriteFile(marker, message, sizeof(message) - 1, &written, nullptr)) {
        std::fputs("failed to write readiness marker\n", stderr);
        std::exit(1);
    }
    CloseHandle(marker);
    MSG event;
    while (GetMessageW(&event, nullptr, 0, 0) > 0) {
        TranslateMessage(&event);
        DispatchMessageW(&event);
    }
}

int main() {
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    wchar_t stage[64] = L"present";
    GetEnvironmentVariableW(L"INCULAR_GPU_PROBE_STAGE", stage, 64);

    WNDCLASSW window_class{};
    window_class.lpfnWndProc = window_proc;
    window_class.hInstance = GetModuleHandleW(nullptr);
    window_class.lpszClassName = L"IncularNativeD3D12MemoryProbe";
    if (!RegisterClassW(&window_class)) {
        std::fputs("failed to register window class\n", stderr);
        return 1;
    }
    RECT bounds{0, 0, 1650, 1080};
    AdjustWindowRectExForDpi(&bounds, WS_OVERLAPPEDWINDOW, FALSE, 0,
                             GetDpiForSystem());
    HWND window = CreateWindowExW(
        0, window_class.lpszClassName, L"Native D3D12 memory probe",
        WS_OVERLAPPEDWINDOW | WS_VISIBLE, CW_USEDEFAULT, CW_USEDEFAULT,
        bounds.right - bounds.left, bounds.bottom - bounds.top, nullptr,
        nullptr, window_class.hInstance, nullptr);
    if (!window) {
        std::fputs("failed to create window\n", stderr);
        return 1;
    }
    RECT client{};
    if (!GetClientRect(window, &client) || client.right != 1650 ||
        client.bottom != 1080) {
        std::fputs("unexpected physical client size\n", stderr);
        return 1;
    }
    std::printf("CLIENT %ldx%ld\n", client.right, client.bottom);
    std::fflush(stdout);
    if (std::wcscmp(stage, L"window") == 0) {
        ready_and_idle();
        return 0;
    }

    ComPtr<IDXGIFactory6> factory;
    require(CreateDXGIFactory2(0, IID_PPV_ARGS(&factory)), "CreateDXGIFactory2");
    ComPtr<IDXGIAdapter1> adapter;
    for (UINT index = 0;; ++index) {
        ComPtr<IDXGIAdapter1> candidate;
        HRESULT result = factory->EnumAdapters1(index, &candidate);
        if (result == DXGI_ERROR_NOT_FOUND) break;
        require(result, "EnumAdapters1");
        DXGI_ADAPTER_DESC1 info{};
        require(candidate->GetDesc1(&info), "GetDesc1");
        if (info.VendorId == 0x1002 && info.DeviceId == 0x164e) {
            adapter = candidate;
            std::printf("ADAPTER vendor=0x%x device=0x%x\n", info.VendorId,
                        info.DeviceId);
            std::fflush(stdout);
            break;
        }
    }
    if (!adapter) {
        std::fputs("AMD Radeon 610M adapter not found\n", stderr);
        return 1;
    }
    ComPtr<ID3D12Device> device;
    require(D3D12CreateDevice(adapter.Get(), D3D_FEATURE_LEVEL_11_0,
                              IID_PPV_ARGS(&device)), "D3D12CreateDevice");
    D3D12_COMMAND_QUEUE_DESC queue_config{};
    queue_config.Type = D3D12_COMMAND_LIST_TYPE_DIRECT;
    ComPtr<ID3D12CommandQueue> queue;
    require(device->CreateCommandQueue(&queue_config, IID_PPV_ARGS(&queue)),
            "CreateCommandQueue");
    if (std::wcscmp(stage, L"device") == 0) {
        ready_and_idle();
        return 0;
    }

    DXGI_SWAP_CHAIN_DESC1 swapchain_config{};
    swapchain_config.Width = 1650;
    swapchain_config.Height = 1080;
    swapchain_config.Format = DXGI_FORMAT_B8G8R8A8_UNORM;
    swapchain_config.SampleDesc.Count = 1;
    swapchain_config.BufferUsage = DXGI_USAGE_RENDER_TARGET_OUTPUT;
    swapchain_config.BufferCount = 2;
    swapchain_config.Scaling = DXGI_SCALING_STRETCH;
    swapchain_config.SwapEffect = DXGI_SWAP_EFFECT_FLIP_DISCARD;
    swapchain_config.AlphaMode = DXGI_ALPHA_MODE_IGNORE;
    swapchain_config.Flags = DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT;
    ComPtr<IDXGISwapChain1> swapchain1;
    require(factory->CreateSwapChainForHwnd(queue.Get(), window,
                                            &swapchain_config, nullptr, nullptr,
                                            &swapchain1), "CreateSwapChainForHwnd");
    ComPtr<IDXGISwapChain2> swapchain;
    require(swapchain1.As(&swapchain), "IDXGISwapChain2");
    require(swapchain->SetMaximumFrameLatency(1), "SetMaximumFrameLatency");
    std::puts("CONFIG 1650x1080 BGRA8 2 buffers flip-discard latency 1");
    std::fflush(stdout);
    if (std::wcscmp(stage, L"configure") == 0) {
        ready_and_idle();
        return 0;
    }
    if (std::wcscmp(stage, L"present") != 0) {
        std::fputs("unknown probe stage\n", stderr);
        return 1;
    }
    for (unsigned frame = 0; frame < 256; ++frame) {
        require(swapchain->Present(1, 0), "Present");
    }
    ready_and_idle();
    return 0;
}
