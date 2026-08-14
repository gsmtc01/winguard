// src/components/icons.tsx
// 목업의 스트로크 SVG 아이콘들을 재사용 가능한 React 컴포넌트로 모았다.
// color 는 currentColor 를 따르므로 text-* 유틸로 색을 제어한다.

type IconProps = {
  size?: number;
  className?: string;
  strokeWidth?: number;
};

const base = (size: number, strokeWidth: number, className?: string) => ({
  width: size,
  height: size,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  className,
});

export const IconDashboard = ({ size = 18, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <rect x="3" y="3" width="7" height="7" rx="1.5" />
    <rect x="14" y="3" width="7" height="7" rx="1.5" />
    <rect x="3" y="14" width="7" height="7" rx="1.5" />
    <rect x="14" y="14" width="7" height="7" rx="1.5" />
  </svg>
);

export const IconProcess = ({ size = 18, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M9 3H5a2 2 0 0 0-2 2v4M15 3h4a2 2 0 0 1 2 2v4M9 21H5a2 2 0 0 1-2-2v-4M15 21h4a2 2 0 0 0 2-2v-4" />
    <rect x="8" y="8" width="8" height="8" rx="1.5" />
  </svg>
);

export const IconNetwork = ({ size = 18, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M5 12h4l2 5 3-12 2 7h3" />
  </svg>
);

export const IconEvents = ({ size = 18, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M14 3v4a1 1 0 0 0 1 1h4" />
    <path d="M5 21V5a2 2 0 0 1 2-2h7l5 5v13a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2z" />
    <path d="M9 13h6M9 17h4" />
  </svg>
);

export const IconExtension = ({ size = 18, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <circle cx="12" cy="12" r="9" />
    <path d="M2 12h20M12 3a14 14 0 0 1 0 18 14 14 0 0 1 0-18z" />
  </svg>
);

export const IconPrivacy = ({ size = 18, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7z" />
    <circle cx="12" cy="12" r="2.5" />
  </svg>
);

export const IconFiles = ({ size = 18, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M4 7l8-4 8 4v6c0 4-3.5 7-8 8-4.5-1-8-4-8-8V7z" />
    <path d="M9 12l2 2 4-4" />
  </svg>
);

export const IconShield = ({ size = 18, className, strokeWidth = 1.8 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M12 3l7 3v5c0 4.4-3 7.5-7 9-4-1.5-7-4.6-7-9V6l7-3z" />
    <path d="M9 12l2 2 4-4" />
  </svg>
);

export const IconShieldAlert = ({ size = 20, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M4 7l8-4 8 4v6c0 4-3.5 7-8 8-4.5-1-8-4-8-8V7z" />
    <path d="M12 8v4M12 16h.01" />
  </svg>
);

export const IconLock = ({ size = 20, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <rect x="4" y="10" width="16" height="11" rx="2" />
    <path d="M8 10V7a4 4 0 0 1 8 0v3" />
  </svg>
);

export const IconInfo = ({ size = 20, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <circle cx="12" cy="12" r="9" />
    <path d="M12 11v5M12 8h.01" />
  </svg>
);

export const IconAlertTriangle = ({ size = 15, className, strokeWidth = 2.2 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M12 9v4M12 17h.01M10.3 3.9L1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" />
  </svg>
);

export const IconSearch = ({ size = 16, className, strokeWidth = 2.1 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <circle cx="11" cy="11" r="7" />
    <path d="M21 21l-4-4" />
  </svg>
);

export const IconSparkle = ({ size = 15, className, strokeWidth = 1.8 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M9.937 15.5A2 2 0 0 0 8.5 14.063l-6.135-1.582a.5.5 0 0 1 0-.962L8.5 9.936A2 2 0 0 0 9.937 8.5l1.582-6.135a.5.5 0 0 1 .962 0L14.063 8.5A2 2 0 0 0 15.5 9.937l6.135 1.581a.5.5 0 0 1 0 .964L15.5 14.063a2 2 0 0 0-1.437 1.437l-1.582 6.135a.5.5 0 0 1-.962 0z" />
  </svg>
);

export const IconSun = ({ size = 16, className, strokeWidth = 1.8 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2M12 20v2M2 12h2M20 12h2M5 5l1.4 1.4M17.6 17.6L19 19M19 5l-1.4 1.4M6.4 17.6L5 19" />
  </svg>
);

export const IconMonitor = ({ size = 16, className, strokeWidth = 1.8 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <rect x="3" y="4" width="18" height="12" rx="2" />
    <path d="M8 20h8M12 16v4" />
  </svg>
);

export const IconMoon = ({ size = 16, className, strokeWidth = 1.8 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M21 12.8A8 8 0 1 1 11.2 3a6.2 6.2 0 0 0 9.8 9.8z" />
  </svg>
);

export const IconSettings = ({ size = 17, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <circle cx="12" cy="12" r="3" />
    <path d="M19.4 15a1.6 1.6 0 0 0 .3 1.8 2 2 0 1 1-2.8 2.8 1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5 2 2 0 1 1-4 0 1.6 1.6 0 0 0-1-1.5 1.6 1.6 0 0 0-1.8.3 2 2 0 1 1-2.8-2.8 1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1 2 2 0 1 1 0-4 1.6 1.6 0 0 0 1.5-1 1.6 1.6 0 0 0-.3-1.8 2 2 0 1 1 2.8-2.8 1.6 1.6 0 0 0 1.8.3 1.6 1.6 0 0 0 1-1.5 2 2 0 1 1 4 0 1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3 2 2 0 1 1 2.8 2.8 1.6 1.6 0 0 0-.3 1.8 1.6 1.6 0 0 0 1.5 1 2 2 0 1 1 0 4 1.6 1.6 0 0 0-1.5 1z" />
  </svg>
);

export const IconClose = ({ size = 18, className, strokeWidth = 1.9 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M6 6l12 12M18 6L6 18" />
  </svg>
);

export const IconChevronRight = ({ size = 14, className, strokeWidth = 2 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M9 6l6 6-6 6" />
  </svg>
);

export const IconCamera = ({ size = 23, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <rect x="2" y="6" width="14" height="12" rx="2" />
    <path d="M16 10l6-3v10l-6-3z" />
  </svg>
);

export const IconMic = ({ size = 23, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <rect x="9" y="3" width="6" height="11" rx="3" />
    <path d="M5 11a7 7 0 0 0 14 0M12 18v3" />
  </svg>
);

export const IconCheckCircle = ({ size = 18, className, strokeWidth = 1.8 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <circle cx="12" cy="12" r="9" />
    <path d="M8 12l2.5 2.5L16 9" />
  </svg>
);

export const IconUpload = ({ size = 30, className, strokeWidth = 1.6 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M12 16V4M8 8l4-4 4 4" />
    <path d="M4 16v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
  </svg>
);

export const IconZap = ({ size = 18, className, strokeWidth = 1.8 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M13 2L3 14h7l-1 8 10-12h-7z" />
  </svg>
);

export const IconList = ({ size = 18, className, strokeWidth = 1.8 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M3 7h18M3 12h18M3 17h12" />
  </svg>
);

export const IconHistory = ({ size = 20, className, strokeWidth = 1.7 }: IconProps) => (
  <svg {...base(size, strokeWidth, className)}>
    <path d="M3 12a9 9 0 1 0 3-6.7" />
    <path d="M3 4v4h4" />
    <path d="M12 7v5l4 2" />
  </svg>
);
