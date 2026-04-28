import { useState } from 'react';
import type { MemberContribution } from './types';
import { format } from 'date-fns';

interface TimelineNodeProps {
  contribution: MemberContribution;
  x: number;
  y: number;
  onClick?: (contribution: MemberContribution) => void;
}

export function TimelineNode({ contribution, x, y, onClick }: TimelineNodeProps) {
  const [hovered, setHovered] = useState(false);

  return (
    <g
      transform={`translate(${x}, ${y})`}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={() => onClick?.(contribution)}
      style={{ cursor: 'pointer' }}
      data-testid={`timeline-node-${contribution.id}`}
    >
      <circle
        r={hovered ? 9 : 7}
        fill={contribution.groupColor}
        stroke="#fff"
        strokeWidth={2}
        opacity={0.92}
      />
      {hovered && (
        <g transform="translate(14, -55)">
          <rect
            x={0}
            y={0}
            width={260}
            height={85}
            rx={8}
            fill="rgba(15, 23, 42, 0.96)"
            stroke={contribution.groupColor}
            strokeWidth={1.5}
          />
          <text x={12} y={22} fill="#fff" fontSize={13} fontWeight={600}>
            {contribution.title}
          </text>
          <text x={12} y={42} fill="#94a3b8" fontSize={11}>
            {contribution.groupName} • {format(contribution.timestamp, 'MMM d, yyyy')}
          </text>
          <text x={12} y={60} fill="#cbd5e1" fontSize={11}>
            {contribution.description.slice(0, 38)}
            {contribution.description.length > 38 ? '…' : ''}
          </text>
          {contribution.amount !== undefined && (
            <text x={12} y={76} fill="#10b981" fontSize={11} fontWeight={600}>
              {contribution.amount} XLM
            </text>
          )}
        </g>
      )}
    </g>
  );
}

