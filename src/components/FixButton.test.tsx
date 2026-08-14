import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { FixButton } from "./FixButton";

const invokeMock = vi.fn();
const openUrlMock = vi.fn().mockResolvedValue(undefined);

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: (...args: unknown[]) => openUrlMock(...args),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

const UAC_REG_PATH = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System";

describe("FixButton", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    openUrlMock.mockClear();
  });

  it("http URI 가 들어오면 버튼을 렌더링하지 않는다", () => {
    const { container } = render(<FixButton uri="http://malicious.com" />);
    expect(container.firstChild).toBeNull();
  });

  it("null 이 들어오면 버튼을 렌더링하지 않는다", () => {
    const { container } = render(<FixButton uri={null} />);
    expect(container.firstChild).toBeNull();
  });

  it("ms-settings URI 는 버튼을 렌더링한다", () => {
    render(<FixButton uri="ms-settings:windowsdefender" />);
    expect(screen.getByRole("button")).toBeInTheDocument();
  });

  it("실행파일 경로는 버튼을 렌더링하지 않는다", () => {
    const { container } = render(<FixButton uri="C:\\Windows\\system32\\cmd.exe" />);
    expect(container.firstChild).toBeNull();
  });

  it("ms-settings URI 클릭 시 openUrl 을 호출한다 (백업 없이 즉시 실행)", async () => {
    render(<FixButton uri="ms-settings:windowsdefender" />);
    fireEvent.click(screen.getByRole("button"));
    await waitFor(() => {
      expect(openUrlMock).toHaveBeenCalledWith("ms-settings:windowsdefender");
    });
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("cmd: URI 클릭 시 레지스트리 원본값을 백업(quarantine_save)한 후 실제 커맨드를 실행한다", async () => {
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === "read_reg_dword") {
        if (args?.name === "EnableLUA") return Promise.resolve(1);
        if (args?.name === "ConsentPromptBehaviorAdmin") return Promise.resolve(5);
      }
      if (cmd === "quarantine_save") return Promise.resolve("173000000000");
      if (cmd === "open_uac_settings") return Promise.resolve();
      return Promise.reject(new Error(`unexpected invoke: ${cmd}`));
    });

    render(<FixButton uri="cmd:uac-settings" />);
    fireEvent.click(screen.getByRole("button"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "quarantine_save",
        expect.objectContaining({
          checkId: "uac-settings",
          actionLabel: "UAC 설정 열기",
          entries: [
            {
              hive: "HKLM",
              path: UAC_REG_PATH,
              name: "EnableLUA",
              value_type: "DWORD",
              original_value: 1,
            },
            {
              hive: "HKLM",
              path: UAC_REG_PATH,
              name: "ConsentPromptBehaviorAdmin",
              value_type: "DWORD",
              original_value: 5,
            },
          ],
        })
      );
    });

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("open_uac_settings");
    });
  });

  it("read_reg_dword 가 실패해도(키 없음 = 정상) fallback 값으로 백업을 계속 진행한다", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "read_reg_dword") return Promise.reject(new Error("키 없음"));
      if (cmd === "quarantine_save") return Promise.resolve("173000000000");
      if (cmd === "open_uac_settings") return Promise.resolve();
      return Promise.reject(new Error(`unexpected invoke: ${cmd}`));
    });

    render(<FixButton uri="cmd:uac-settings" />);
    fireEvent.click(screen.getByRole("button"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith(
        "quarantine_save",
        expect.objectContaining({
          entries: [
            expect.objectContaining({ name: "EnableLUA", original_value: 1 }),
            expect.objectContaining({ name: "ConsentPromptBehaviorAdmin", original_value: 5 }),
          ],
        })
      );
    });
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("open_uac_settings");
    });
  });

  it("quarantine_save 가 실패하면 수정을 중단하고 오류를 표시한다 (§4-11)", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "read_reg_dword") return Promise.resolve(1);
      if (cmd === "quarantine_save") return Promise.reject(new Error("디스크 쓰기 실패"));
      return Promise.reject(new Error(`unexpected invoke: ${cmd}`));
    });

    render(<FixButton uri="cmd:uac-settings" />);
    fireEvent.click(screen.getByRole("button"));

    await waitFor(() => {
      expect(screen.getByText(/백업 실패로 수정이 중단되었습니다/)).toBeInTheDocument();
    });
    expect(invokeMock).not.toHaveBeenCalledWith("open_uac_settings");
  });
});
