// src/components/AiSlideOver.tsx
// 우측 슬라이드오버 — 토pbar "AI 분석" 버튼으로 열린다.
// 본문은 기존 LlmPanel(종합분석/로드맵/Q&A)을 호스팅한다.

import { useCheckStore, type ScanReport } from "@/store/checkStore";
import { useUIStore } from "@/store/uiStore";
import { LlmPanel } from "./LlmPanel";
import { IconSparkle, IconClose } from "./icons";

export const AiSlideOver = () => {
  const { aiOpen, closeAi } = useUIStore();
  const { report } = useCheckStore();

  return (
    <>
      {/* 백드롭 */}
      <div
        onClick={closeAi}
        aria-hidden="true"
        className="absolute inset-0 z-20 bg-black/40 transition-opacity duration-300"
        style={{ opacity: aiOpen ? 1 : 0, pointerEvents: aiOpen ? "auto" : "none" }}
      />

      {/* 패널 */}
      <aside
        className="absolute inset-y-0 right-0 z-30 flex w-[440px] flex-col border-l border-border bg-surface shadow-[-14px_0_40px_rgba(0,0,0,.18)]"
        style={{
          transform: aiOpen ? "translateX(0)" : "translateX(100%)",
          transition: "transform .32s cubic-bezier(.4,0,.2,1)",
        }}
      >
        {/* 헤더 */}
        <div className="flex h-[60px] flex-none items-center gap-[11px] border-b border-border px-[22px]">
          <span className="flex h-[30px] w-[30px] items-center justify-center rounded-lg bg-accent-soft text-accent">
            <IconSparkle size={17} />
          </span>
          <div className="flex-1">
            <div className="text-[15px] font-extrabold text-text">AI 분석</div>
            <div className="font-mono text-[11px] text-text-3">기기 내 추론 · 외부 전송 없음</div>
          </div>
          <button
            onClick={closeAi}
            aria-label="닫기"
            className="flex h-8 w-8 items-center justify-center rounded-lg text-text-2 transition hover:bg-hover"
          >
            <IconClose size={18} />
          </button>
        </div>

        {/* 본문 */}
        <div data-scroll className="flex-1 overflow-y-auto px-[22px] py-5">
          {report ? (
            <LlmPanel report={report as ScanReport} />
          ) : (
            <p className="mt-8 text-center text-sm text-text-3">
              먼저 보안 대시보드에서 검사를 실행하면
              <br />
              AI 분석을 사용할 수 있습니다.
            </p>
          )}
        </div>
      </aside>
    </>
  );
};
