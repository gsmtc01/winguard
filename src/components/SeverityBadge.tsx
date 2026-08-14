import { severityConfig } from "@/lib/severity";
import type { Severity } from "@/lib/severity";

export const SeverityBadge = ({ severity }: { severity: Severity }) => {
  const cfg = severityConfig[severity];
  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-semibold ${cfg.bg} ${cfg.text}`}
    >
      <span className={`w-1.5 h-1.5 rounded-full ${cfg.dot}`} />
      {cfg.label}
    </span>
  );
};
