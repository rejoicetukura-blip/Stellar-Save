import { useMemo } from 'react';
import { MemberContribution, TimelineFilters } from './types';
import { isWithinInterval, startOfDay, endOfDay } from 'date-fns';

export function useFilteredContributions(
  contributions: MemberContribution[],
  filters: TimelineFilters,
): MemberContribution[] {
  return useMemo(() => {
    return contributions
      .filter((c) => {
        if (filters.groupIds.length > 0 && !filters.groupIds.includes(c.groupId)) {
          return false;
        }
        if (filters.eventTypes.length > 0 && !filters.eventTypes.includes(c.type)) {
          return false;
        }
        if (filters.dateRange.start || filters.dateRange.end) {
          const interval = {
            start: filters.dateRange.start ? startOfDay(filters.dateRange.start) : new Date(0),
            end: filters.dateRange.end ? endOfDay(filters.dateRange.end) : new Date(8640000000000000),
          };
          if (!isWithinInterval(c.timestamp, interval)) {
            return false;
          }
        }
        return true;
      })
      .sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime());
  }, [contributions, filters]);
}

