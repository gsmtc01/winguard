// src/components/Toggle.tsx
// 토큰 기반 공통 슬라이드 스위치 (목업 swVals 스타일).

type ToggleProps = {
  checked: boolean;
  onChange: () => void;
  disabled?: boolean;
  "aria-label"?: string;
};

export const Toggle = ({ checked, onChange, disabled, ...rest }: ToggleProps) => (
  <button
    type="button"
    role="switch"
    aria-checked={checked}
    aria-label={rest["aria-label"]}
    onClick={onChange}
    disabled={disabled}
    className={`relative h-6 w-[42px] flex-none rounded-full transition-all duration-200 disabled:opacity-50 ${
      checked ? "bg-accent opacity-100" : "bg-text-3 opacity-50"
    }`}
  >
    <span
      className="absolute top-[3px] h-[18px] w-[18px] rounded-full bg-white shadow-[0_1px_3px_rgba(0,0,0,.35)] transition-[left] duration-200"
      style={{ left: checked ? 21 : 3 }}
    />
  </button>
);
