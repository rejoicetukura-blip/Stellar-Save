export interface MemberContribution {
  id: string;
  memberAddress: string;
  groupId: string;
  groupName: string;
  groupColor: string;
  title: string;
  description: string;
  timestamp: Date;
  type: 'contribution' | 'payout' | 'member_join' | 'cycle_complete';
  amount?: number;
  transactionHash?: string;
  status: 'completed' | 'pending' | 'failed';
}

export interface TimelineFilters {
  groupIds: string[];
  dateRange: {
    start: Date | null;
    end: Date | null;
  };
  eventTypes: MemberContribution['type'][];
}

export const DEFAULT_TIMELINE_FILTERS: TimelineFilters = {
  groupIds: [],
  dateRange: { start: null, end: null },
  eventTypes: [],
};

