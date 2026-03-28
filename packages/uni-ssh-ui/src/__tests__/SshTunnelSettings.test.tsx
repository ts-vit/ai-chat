import { describe, it, expect, vi, beforeAll, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SettingsProvider } from "@uni-fw/ui";
import type { SettingsAdapter } from "@uni-fw/ui";
import { SshTunnelSettings } from "../SshTunnelSettings";
import { MantineProvider } from "@mantine/core";

// Mock Tauri APIs
const mockInvoke = vi.fn();
const mockListen = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => mockListen(...args),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

vi.mock("@mantine/notifications", () => ({
  Notifications: () => null,
  notifications: { show: vi.fn(), clean: vi.fn() },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn(), language: "en" },
  }),
}));

beforeAll(() => {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: query === "(prefers-color-scheme: dark)",
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });

  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
});

function makeMockAdapter(
  initial: Record<string, string> = {}
): SettingsAdapter {
  const store = { ...initial };
  return {
    get: vi.fn(async (key: string) => store[key]),
    set: vi.fn(async (key: string, value: string) => {
      store[key] = value;
    }),
    delete: vi.fn(async (key: string) => {
      delete store[key];
    }),
    getAll: vi.fn(async () =>
      Object.entries(store).map(([key, value]) => ({
        key,
        value,
        isSensitive: false,
      }))
    ),
  };
}

function renderWithProvider(
  adapter: SettingsAdapter,
  props: { extraConnectParams?: Record<string, unknown>; proxyUrlOverride?: string | null } = {},
) {
  return render(
    <MantineProvider>
      <SettingsProvider adapter={adapter}>
        <SshTunnelSettings {...props} />
      </SettingsProvider>
    </MantineProvider>
  );
}

beforeEach(() => {
  mockInvoke.mockReset();
  mockListen.mockReset();
  // ssh_tunnel_status returns disconnected by default
  mockInvoke.mockResolvedValue({ connected: false, localPort: null, remoteHost: null });
  // listen returns unsubscribe function
  mockListen.mockResolvedValue(() => {});
});

describe("SshTunnelSettings", () => {
  it("renders with defaults — shows host/port/username fields and Connect button", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("settings.vpn.title")).toBeInTheDocument();
    });

    expect(screen.getByText("settings.vpn.ssh.host")).toBeInTheDocument();
    expect(screen.getByText("settings.vpn.ssh.port")).toBeInTheDocument();
    expect(screen.getByText("settings.vpn.ssh.username")).toBeInTheDocument();
    expect(screen.getByText("settings.vpn.ssh.connect")).toBeInTheDocument();
  });

  it("shows password field by default, key path field when auth type is key", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("settings.vpn.ssh.password")).toBeInTheDocument();
    });

    // Key path should not be visible
    expect(screen.queryByText("settings.vpn.ssh.keyPath")).not.toBeInTheDocument();
  });

  it("shows key path field when auth type is key", async () => {
    const adapter = makeMockAdapter({
      "ssh.auth_type": "key",
    });
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("settings.vpn.ssh.keyPath")).toBeInTheDocument();
    });

    expect(screen.queryByText("settings.vpn.ssh.password")).not.toBeInTheDocument();
  });

  it("calls invoke ssh_tunnel_connect when Connect is clicked", async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "ssh_tunnel_status") {
        return { connected: false, localPort: null, remoteHost: null };
      }
      if (cmd === "ssh_tunnel_connect") {
        return 1080;
      }
      return null;
    });

    const adapter = makeMockAdapter({
      "ssh.host": "example.com",
      "ssh.port": "22",
      "ssh.username": "root",
      "ssh.auth_type": "password",
      "ssh.password": "secret",
    });
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("settings.vpn.ssh.connect")).toBeInTheDocument();
    });

    const connectBtn = screen.getByText("settings.vpn.ssh.connect").closest("button")!;

    // Wait for settings to load so button is enabled
    await waitFor(() => {
      expect(connectBtn).not.toBeDisabled();
    });

    fireEvent.click(connectBtn);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("ssh_tunnel_connect", {
        host: "example.com",
        port: 22,
        username: "root",
        authType: "password",
        password: "secret",
        privateKey: null,
      });
    });
  });

  it("saves host onBlur via adapter", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(screen.getByText("settings.vpn.ssh.host")).toBeInTheDocument();
    });

    const hostInput = screen.getByPlaceholderText("settings.vpn.ssh.hostPlaceholder");
    fireEvent.change(hostInput, { target: { value: "newhost.com" } });
    fireEvent.blur(hostInput);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith("ssh.host", "newhost.com");
    });
  });

  it("passes extraConnectParams to ssh_tunnel_connect", async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "ssh_tunnel_status") {
        return { connected: false, localPort: null, remoteHost: null };
      }
      if (cmd === "ssh_tunnel_connect") {
        return 1080;
      }
      return null;
    });

    const adapter = makeMockAdapter({
      "ssh.host": "example.com",
      "ssh.port": "22",
      "ssh.username": "root",
      "ssh.auth_type": "password",
      "ssh.password": "secret",
    });
    renderWithProvider(adapter, {
      extraConnectParams: { forwardRemoteHost: "127.0.0.1", forwardRemotePort: 8888 },
    });

    await waitFor(() => {
      expect(screen.getByText("settings.vpn.ssh.connect")).toBeInTheDocument();
    });

    const connectBtn = screen.getByText("settings.vpn.ssh.connect").closest("button")!;
    await waitFor(() => {
      expect(connectBtn).not.toBeDisabled();
    });

    fireEvent.click(connectBtn);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("ssh_tunnel_connect", {
        host: "example.com",
        port: 22,
        username: "root",
        authType: "password",
        password: "secret",
        privateKey: null,
        forwardRemoteHost: "127.0.0.1",
        forwardRemotePort: 8888,
      });
    });
  });

  it("shows proxyUrlOverride when connected", async () => {
    mockInvoke.mockImplementation(async (cmd: string) => {
      if (cmd === "ssh_tunnel_status") {
        return { connected: true, localPort: 34083, remoteHost: "example.com" };
      }
      return null;
    });

    const adapter = makeMockAdapter();
    renderWithProvider(adapter, { proxyUrlOverride: "http://127.0.0.1:34083" });

    await waitFor(() => {
      expect(screen.getByText("http://127.0.0.1:34083")).toBeInTheDocument();
    });

    expect(screen.queryByText(/socks5:/)).not.toBeInTheDocument();
  });

  it("toggles auto-connect switch and saves immediately", async () => {
    const adapter = makeMockAdapter();
    renderWithProvider(adapter);

    await waitFor(() => {
      expect(
        screen.getByText("settings.vpn.ssh.autoConnect")
      ).toBeInTheDocument();
    });

    const switchEl = screen.getByRole("switch");
    fireEvent.click(switchEl);

    await waitFor(() => {
      expect(adapter.set).toHaveBeenCalledWith("ssh.auto_connect", "true");
    });
  });
});
