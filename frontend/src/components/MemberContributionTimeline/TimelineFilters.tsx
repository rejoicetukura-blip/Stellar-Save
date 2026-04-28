import { useCallback } from 'react';
import type { MemberContribution, TimelineFilters } from './types';

interface UniqueGroup {
  id: string;
  name: string;
  color: string;
}

interface TimelineFiltersProps {
  contributions: MemberContribution[];
  filters: TimelineFilters;
  onChange: (filters: TimelineFilters) => void;
}

function getUniqueGroups(contributions: MemberContribution[]): UniqueGroup[] {
  const map = new Map<string, UniqueGroup>();
  for (const c of contributions) {
    if (!map.has(c.groupId)) {
      map.set(c.groupId, { id: c.groupId, name: c.groupName, color: c.groupColor });
    }
  }
  return Array.from(map.values());
}

const EVENT_TYPE_LABELS: Record<MemberContribution['type'], string> = {
  contribution: 'Contributions',
  payout: 'Payouts',
  member_join: 'Joined',
  cycle_complete: 'Cycle Complete',
};

export function TimelineFiltersPanel({ contributions, filters, onChange }: TimelineFiltersProps) {
  const groups = getUniqueGroups(contributions);

  const toggleGroup = useCallback(
    (groupId: string) => {
      const next = filters.groupIds.includes(groupId)
        ? filters.groupIds.filter((id) => id !== groupId)
        : [...filters.groupIds, groupId];
      onChange({ ...filters, groupIds: next });
    },
    [filters, onChange],
  );

  const toggleEventType = useCallback(
    (type: MemberContribution['type']) => {
      const next = filters.eventTypes.includes(type)
        ? filters.eventTypes.filter((t) => t !== type)
        : [...filters.eventTypes, type];
      onChange({ ...filters, eventTypes: next });
    },
    [filters, onChange],
  );

  const setStart = useCallback(
    (value: string) => {
      onChange({
        ...filters,
        dateRange: { ...filters.dateRange, start: value ? new Date(value) : null },
      });
    },
    [filters, onChange],
  );

  const setEnd = useCallback(
    (value: string) => {
      onChange({
        ...filters,
        dateRange: { ...filters.dateRange, end: value ? new Date(value) : null },
      });
    },
    [filters, onChange],
  );

  const formatInputDate = (date: Date | null) =>
    date ? date.toISOString().split('T')[0] : '';

  const clearFilters = useCallback(() => {
    onChange({ groupIds: [], dateRange: { start: null, end: null }, eventTypes: [] });
  }, [onChange]);

  const hasActiveFilters =
    filters.groupIds.length > 0 ||
    filters.eventTypes.length > 0 ||
    filters.dateRange.start !== null ||
    filters.dateRange.end !== null;

  return (
    <div className="mct-filters" data-testid="timeline-filters">
      <div className="mct-filter-section">
        <span className="mct-filter-label">Groups</span>
        <div className="mct-filter-chips">
          {groups.map((group) => (
            <button
              key={group.id}
              className={`mct-chip ${filters.groupIds.includes(group.id) ? 'active' : ''}`}
              style={{ '--group-color': group.color } as React.CSSProperties}
              onClick={() => toggleGroup(group.id)}
              data-testid={`group-filter-${group.id}`}
            >
              {group.name}
            </button>
          ))}
        </div>
      </div>

      <div className="mct-filter-section">
        <span className="mct-filter-label">Event Types</span>
        <div className="mct-filter-chips">
          {(Object.keys(EVENT_TYPE_LABELS) as MemberContribution['type'][]).map((type) => (
            <button
              key={type}
              className={`mct-chip mct-chip--type ${filters.eventTypes.includes(type) ? 'active' : ''}`}
              onClick={() => toggleEventType(type)}
              data-testid={`type-filter-${type}`}
            >
              {EVENT_TYPE_LABELS[type]}
            </button>
          ))}
        </div>
      </div>

      <div className="mct-filter-section">
        <span className="mct-filter-label">Date Range</span>
        <div className="mct-date-range">
          <input
            type="date"
            value={formatInputDate(filters.dateRange.start)}
            onChange={(e) => setStart(e.target.value)}
            data-testid="date-start"
          />
          <span className="mct-range-separator">to</span>
          <input
            type="date"
            value={formatInputDate(filters.dateRange.end)}
            onChange={(e) => setEnd(e.target.value)}
            data-testid="date-end"
          />
        </div>
      </div>

      {hasActiveFilters && (
        <button className="mct-clear-btn" onClick={clearFilters} data-testid="clear-filters">
          Clear Filters
        </button>
      )}
    </div>
  );
}

