import { useMemo } from 'react';
import * as d3 from 'd3';

interface TimelineAxisProps {
  scale: d3.ScaleTime<number, number>;
  height: number;
  transform?: string;
}

export function TimelineAxis({ scale, height, transform }: TimelineAxisProps) {
  const ticks = useMemo(() => {
    const [start, end] = scale.domain();
    const ms = end.getTime() - start.getTime();
    const days = ms / (1000 * 60 * 60 * 24);

    let interval: d3.TimeInterval;
    if (days <= 7) interval = d3.timeDay;
    else if (days <= 60) interval = d3.timeWeek;
    else if (days <= 365) interval = d3.timeMonth;
    else interval = d3.timeYear;

    return scale.ticks(interval);
  }, [scale]);

  const formatTick = (date: Date) => {
    const [start, end] = scale.domain();
    const days = (end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24);

    if (days <= 7) return d3.timeFormat('%b %d')(date);
    if (days <= 60) return d3.timeFormat('%b %d')(date);
    if (days <= 365) return d3.timeFormat('%b')(date);
    return d3.timeFormat('%Y')(date);
  };

  return (
    <g transform={transform} data-testid="timeline-axis">
      <line
        x1={scale.range()[0]}
        x2={scale.range()[1]}
        y1={height}
        y2={height}
        stroke="#cbd5e1"
        strokeWidth={1}
      />
      {ticks.map((tick, i) => {
        const x = scale(tick);
        return (
          <g key={i} transform={`translate(${x}, ${height})`}>
            <line y2={6} stroke="#cbd5e1" strokeWidth={1} />
            <text
              y={20}
              textAnchor="middle"
              fill="#64748b"
              fontSize={11}
              fontFamily="system-ui, sans-serif"
            >
              {formatTick(tick)}
            </text>
          </g>
        );
      })}
    </g>
  );
}

