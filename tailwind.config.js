/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "Pretendard",
          "Apple SD Gothic Neo",
          "Malgun Gothic",
          "ui-sans-serif",
          "system-ui",
          "sans-serif",
        ],
        // 숫자·영문은 모노스페이스로, 한글 글리프는 Pretendard로 폴백한다.
        // (ui-monospace/Consolas 에는 한글이 없어 Malgun 등으로 떨어지면
        //  Pretendard 와 톤이 어긋나 "폰트가 온전치 않게" 보이는 문제 방지)
        mono: [
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "Consolas",
          "Pretendard",
          "Apple SD Gothic Neo",
          "Malgun Gothic",
          "monospace",
        ],
      },
      // 디자인 토큰 → Tailwind 색상 유틸 매핑.
      // 컴포넌트는 bg-surface / text-text-2 / border-border / bg-accent 처럼 쓰고
      // 실제 색은 settingsStore 가 :root 의 CSS 변수로 주입한다(테마·액센트 전환).
      colors: {
        bg: "var(--bg)",
        surface: "var(--surface)",
        "surface-2": "var(--surface-2)",
        border: "var(--border)",
        text: {
          DEFAULT: "var(--text)",
          2: "var(--text-2)",
          3: "var(--text-3)",
        },
        accent: {
          DEFAULT: "var(--accent)",
          soft: "var(--accent-soft)",
          ink: "var(--accent-ink)",
        },
        ok: { DEFAULT: "var(--ok)", bg: "var(--ok-bg)" },
        warn: { DEFAULT: "var(--warn)", bg: "var(--warn-bg)" },
        danger: { DEFAULT: "var(--danger)", bg: "var(--danger-bg)" },
        info: { DEFAULT: "var(--info)", bg: "var(--info-bg)" },
        score: { DEFAULT: "var(--score)", track: "var(--score-track)" },
        hover: "var(--hover)",
        rail: { DEFAULT: "var(--rail)", border: "var(--rail-border)" },
      },
    },
  },
  plugins: [],
};
