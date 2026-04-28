import { useRef, useState, useEffect, useCallback } from 'react';
import * as d3 from 'd3';

interface ZoomTransform {
  k: number;
  x: number;
  y: number;
}

export function useTimelineZoom(
  svgRef: React.RefObject<SVGSVGElement | null>,
  width: number,
  height: number,
) {
  const [transform, setTransform] = useState<ZoomTransform>({ k: 1, x: 0, y: 0 });
  const zoomBehaviorRef = useRef<d3.ZoomBehavior<SVGSVGElement, unknown> | null>(null);

  const resetZoom = useCallback(() => {
    if (svgRef.current && zoomBehaviorRef.current) {
      d3.select(svgRef.current)
        .transition()
        .duration(750)
        .call(zoomBehaviorRef.current.transform, d3.zoomIdentity);
    }
  }, [svgRef]);

  useEffect(() => {
    if (!svgRef.current) return;

    const svg = d3.select(svgRef.current);
    const zoom = d3
      .zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.3, 20])
      .translateExtent([
        [-width * 4, -height * 4],
        [width * 4, height * 4],
      ])
      .on('zoom', (event) => {
        setTransform(event.transform);
      });

    zoomBehaviorRef.current = zoom;
    svg.call(zoom);

    return () => {
      svg.on('.zoom', null);
    };
  }, [svgRef, width, height]);

  return { transform, resetZoom };
}

