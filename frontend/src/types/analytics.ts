export interface ContributionDataPoint {
  month: string; // e.g. "Jan 2025"
  contributed: number; // XLM
  received: number; // XLM
}

export interface MemberComparisonItem {
  address: string;
  label: string; // shortened address or "You"
  onTimePercent: number;
  totalContributed: number;
}

export interface AnalyticsStats {
  totalContributed: number;
  totalReceived: number;
  roi: number; // percentage
  onTimePercent: number;
  activeGroups: number;
  completedGroups: number;
}

export interface AnalyticsData {
  stats: AnalyticsStats;
  history: ContributionDataPoint[];
  memberComparison: MemberComparisonItem[];
  isLoading: boolean;
  error: string | null;
}

// ─── Group Analytics ──────────────────────────────────────────────────────────

/** Contribution rate for a single completed cycle */
export interface CycleRate {
  cycleNumber: number;
  contributorsInCycle: number;
  totalMembersInCycle: number;
  /** Percentage 0–100, rounded to one decimal place */
  rate: number;
}

/** Return type of useGroupAnalytics */
export interface GroupAnalyticsResult {
  /** Per-cycle contribution rates for all completed cycles */
  cycleRates: CycleRate[];
  /** On-time payment percentage (0–100), null while loading */
  onTimePercent: number | null;
  /** Projected completion date, null if group not started */
  projectedCompletionDate: Date | null;
  isLoading: boolean;
  error: string | null;
}
