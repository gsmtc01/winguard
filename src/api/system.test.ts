import { describe, it, expect, vi, beforeEach } from "vitest";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import {
  getStartupEnabled,
  setStartupEnabled,
  readRegDword,
  runSettingsCommand,
} from "./system";

describe("api/system", () => {
  beforeEach(() => {
    invokeMock.mockReset();
  });

  it("getStartupEnabled 는 인자 없이 get_startup_enabled 를 호출한다", async () => {
    invokeMock.mockResolvedValue(true);
    await expect(getStartupEnabled()).resolves.toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("get_startup_enabled");
  });

  it("setStartupEnabled 는 enable 인자를 그대로 전달한다", async () => {
    invokeMock.mockResolvedValue(undefined);
    await setStartupEnabled(true);
    expect(invokeMock).toHaveBeenCalledWith("set_startup_enabled", { enable: true });

    await setStartupEnabled(false);
    expect(invokeMock).toHaveBeenLastCalledWith("set_startup_enabled", { enable: false });
  });

  it("readRegDword 는 hive/path/name 을 그대로 넘긴다", async () => {
    invokeMock.mockResolvedValue(1);
    const query = { hive: "HKLM", path: "SOFTWARE\\Test", name: "EnableLUA" };
    await expect(readRegDword(query)).resolves.toBe(1);
    expect(invokeMock).toHaveBeenCalledWith("read_reg_dword", query);
  });

  it("readRegDword 는 키가 없으면 reject 를 그대로 전파한다", async () => {
    invokeMock.mockRejectedValue("not found");
    await expect(readRegDword({ hive: "HKLM", path: "X", name: "Y" })).rejects.toBe("not found");
  });

  it("runSettingsCommand 는 커맨드명을 그대로 invoke 한다", async () => {
    invokeMock.mockResolvedValue(undefined);
    await runSettingsCommand("open_uac_settings");
    expect(invokeMock).toHaveBeenCalledWith("open_uac_settings");
  });
});
