import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import "@testing-library/jest-dom";
import { AreaStatusGrid } from "./Dashboard";
import type { CheckResult } from "@/store/checkStore";
import type { Severity } from "@/lib/severity";

// Dashboard 는 모듈 최상단에서 Tauri API 를 import 한다. 브라우저 밖(jsdom)에서는
// IPC 가 없으므로 여기서 막아둔다 — 이 테스트는 순수 렌더링만 본다.
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: vi.fn(), open: vi.fn() }));

const check = (id: string, severity: Severity): CheckResult => ({
  id,
  title: id,
  severity,
  message: "메시지",
  action_uri: null,
  evidence: {},
  checked_at: 0,
});

describe("AreaStatusGrid", () => {
  it("심각도가 아니라 보안 영역을 카드 제목으로 쓴다", () => {
    render(
      <AreaStatusGrid
        checks={[check("windows_defender", "ok"), check("uac", "warning"), check("firewall", "danger")]}
      />
    );

    expect(screen.getByText("악성코드 방어")).toBeInTheDocument();
    expect(screen.getByText("계정 · 로그인")).toBeInTheDocument();
    expect(screen.getByText("네트워크 · 원격")).toBeInTheDocument();

    // 심각도 분류를 카드 제목으로 반복하지 않는다 (상단 정보와 중복되던 부분)
    expect(screen.queryByText("정상")).not.toBeInTheDocument();
    expect(screen.queryByText("심각")).not.toBeInTheDocument();
  });

  it("영역의 최악 심각도와 조치 건수를 보여준다", () => {
    render(
      <AreaStatusGrid
        checks={[check("firewall", "critical"), check("rdp", "warning"), check("smb1", "ok")]}
      />
    );

    expect(screen.getByText("심각 2건")).toBeInTheDocument();
    expect(screen.getByText("1/3 정상")).toBeInTheDocument();
  });

  it("문제가 없는 영역은 '이상 없음'으로 표시한다", () => {
    render(<AreaStatusGrid checks={[check("bitlocker", "ok"), check("tpm", "ok")]} />);

    expect(screen.getByText("이상 없음")).toBeInTheDocument();
    expect(screen.getByText("2/2 정상")).toBeInTheDocument();
  });

  it("점검 항목이 없으면 아무것도 그리지 않는다", () => {
    const { container } = render(<AreaStatusGrid checks={[]} />);
    expect(container).toBeEmptyDOMElement();
  });
});
