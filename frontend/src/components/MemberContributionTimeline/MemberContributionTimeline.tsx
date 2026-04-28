import { useRef, useMemo, useState, useCallback } from 'react';
import * as d3 from 'd3';
import type { MemberContribution, TimelineFilters } from './types';
import { useTimelineZoom } from './useTimelineZoom';
import { useFilteredContributions } from './useFilteredContributions';
import { TimelineFiltersPanel } from './TimelineFilters';
import { TimelineAxis } from './TimelineAxis';
import { TimelineNode } from './TimelineNode';
import './MemberContributionTimeline.css';

interface MemberContributionTimelineProps {
  contributions: MemberContribution[];
  memberName?: string;
  width?: number;
  height?: number;
}

const PADDING = { top: 40, right: 40, bottom: 60, left: 40 };

export function MemberContributionTimeline({
  contributions,
  memberName,
  width = 960,
  height = 420,
}: MemberContributionTimelineProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const innerWidth = width - PADDING.left - PADDING.right;
  const innerHeight = height - PADDING.top - PADDING.bottom;

  const [filters, setFilters] = useState<TimelineFilters>({
    groupIds: [],
    dateRange: { start: null, end: null },
    eventTypes: [],
  });

  const [selectedEvent, setSelectedEvent] = useState<MemberContribution | null>(null);

  const filtered = useFilteredContributions(contributions, filters);
  const { transform, resetZoom } = useTimelineZoom(svgRef, width, height);

  const timeScale = useMemo(() => {
    if (filtered.length === 0) {
      const now = new Date();
      return d3.scaleTime().domain([now, now]).range([0, innerWidth]);
    }
    const dates = filtered.map((c) => c.timestamp);
    const minDate = d3.min(dates) ?? new Date();
    const maxDate = d3.max(dates) ?? new Date();
    const paddingTime = (maxDate.getTime() - minDate.getTime()) * 0.05 || 86400000;
    return d3
      .scaleTime()
      .domain([
        new Date(minDate.getTime() - paddingTime),
        new Date(maxDate.getTime() + paddingTime),
      ])
      .range([0, innerWidth]);
  }, [filtered, innerWidth]);

  const groupScale = useMemo(() => {
    const groupIds = Array.from(new Set(contributions.map((c) => c.groupId)));
    return d3
      .scalePoint<string>()
      .domain(groupIds)
      .range([0, innerHeight])
      .padding(0.3);
  }, [contributions, innerHeight]);

  const transformedScale = useMemo(() => {
    const newScale = timeScale.copy();
    newScale.range(timeScale.range().map((d) => d * transform.k + transform.x));
    return newScale;
  }, [timeScale, transform]);

  const handleNodeClick = useCallback((event: MemberContribution) => {
    setSelectedEvent(event);
  }, []);

  const uniqueGroups = useMemo(() => {
    const map = new Map<string, { name: string; color: string }>();
    for (const c of contributions) {
      if (!map.has(c.groupId)) {
        map.set(c.groupId, { name: c.groupName, color: c.groupColor });
      }
    }
    return map;
  }, [contributions]);

  return (
    <div className="mct-wrapper" data-testid="member-contribution-timeline">
      <div className="mct-header">
        <div>
          <h2 className="mct-title">
            {memberName ? `${memberName}'s Contribution History` : 'Contribution Timeline'}
          </h2>
          <p className="mct-subtitle">
            {filtered.length} event{filtered.length !== 1 ? 's' : ''} across{' '}
            {uniqueGroups.size} group{uniqueGroups.size !== 1 ? 's' : ''}
          </p>
        </div>
        <button className="mct-reset-btn" onClick={resetZoom} data-testid="reset-zoom">
          Reset Zoom
        </button>
      </div>

      <TimelineFiltersPanel
        contributions={contributions}
        filters={filters}
        onChange={setFilters}
      />

      <div className="mct-svg-wrapper">
        <svg
          ref={svgRef}
          width={width}
          height={height}
          className="mct-svg"
          data-testid="timeline-svg"
        >
          <g transform={`translate(${PADDING.left}, ${PADDING.top})`}>
            {/* Group row lines */}
            {Array.from(uniqueGroups.entries()).map(([groupId, group]) => (
              <g key={groupId}>
                <line
                  x1={0}
                  x2={innerWidth}
                  y1={groupScale(groupId) ?? 0}
                  y2={groupScale(groupId) ?? 0}
                  stroke="#e2e8f0"
                  strokeDasharray="4 4"
                />
                <text
                  x={-10}
                  y={(groupScale(groupId) ?? 0) + 4}
                  textAnchor="end"
                  fill={group.color}
                  fontSize={12}
                  fontWeight={600}
                >
                  {group.name}
                </text>
              </g>
            ))}

            <TimelineAxis scale={transformedScale} height={innerHeight} />

            {filtered.map((contribution) => {
              const x = transformedScale(contribution.timestamp);
              const y = groupScale(contribution.groupId);
              if (x == null || y == null) return null;
              return (
                <TimelineNode
                  key={contribution.id}
                  contribution={contribution}
                  x={x}
                  y={y}
                  onClick={handleNodeClick}
                />
              );
            })}
          </g>
        </svg>
      </div>

      {selectedEvent && (
        <div className="mct-detail" data-testid="event-detail">
          <button
            className="mct-detail-close"
            onClick={() => setSelectedEvent(null)}
            data-testid="close-detail"
          >
            ×
          </button>
          <h3>{selectedEvent.title}</h3>
          <p className="mct-detail-meta">
            {selectedEvent.groupName} • {selectedEvent.timestamp.toLocaleDateString()} •{' '}
            {selectedEvent.type}
          </p>
          <p>{selectedEvent.description}</p>
          {selectedEvent.amount !== undefined && (
            <p className="mct-detail-amount">{selectedEvent.amount} XLM</p>
          )}
          {selectedEvent.transactionHash && (
            <p className="mct-detail-hash">Tx: {selectedEvent.transactionHash}</p>
          )}
        </div>
      )}
    </div>
  );
}

