import { describe, it, expect } from 'vitest';
import { renderHook } from '@testing-library/react';
import { useFilteredContributions } from '../components/MemberContributionTimeline/useFilteredContributions';
import { TimelineFilters, MemberContribution } from '../components/MemberContributionTimeline/types';

const mockData: MemberContribution[] = [
  {
    id: '1',
    memberAddress: 'addr1',
    groupId: 'g1',
    groupName: 'Group A',
    groupColor: '#3b82f6',
    title: 'Event 1',
    description: 'Desc 1',
    timestamp: new Date('2024-03-15'),
    type: 'contribution',
    status: 'completed',
  },
  {
    id: '2',
    memberAddress: 'addr1',
    groupId: 'g2',
    groupName: 'Group B',
    groupColor: '#10b981',
    title: 'Event 2',
    description: 'Desc 2',
    timestamp: new Date('2024-04-20'),
    type: 'payout',
    status: 'completed',
  },
  {
    id: '3',
    memberAddress: 'addr1',
    groupId: 'g1',
    groupName: 'Group A',
    groupColor: '#3b82f6',
    title: 'Event 3',
    description: 'Desc 3',
    timestamp: new Date('2024-05-10'),
    type: 'member_join',
    status: 'completed',
  },
];

describe('useFilteredContributions', () => {
  it('returns all contributions when no filters', () => {
    const filters: TimelineFilters = { groupIds: [], dateRange: { start: null, end: null }, eventTypes: [] };
    const { result } = renderHook(() => useFilteredContributions(mockData, filters));
    expect(result.current).toHaveLength(3);
  });

  it('filters by group', () => {
    const filters: TimelineFilters = { groupIds: ['g1'], dateRange: { start: null, end: null }, eventTypes: [] };
    const { result } = renderHook(() => useFilteredContributions(mockData, filters));
    expect(result.current).toHaveLength(2);
    expect(result.current.every((c) => c.groupId === 'g1')).toBe(true);
  });

  it('filters by event type', () => {
    const filters: TimelineFilters = { groupIds: [], dateRange: { start: null, end: null }, eventTypes: ['payout'] };
    const { result } = renderHook(() => useFilteredContributions(mockData, filters));
    expect(result.current).toHaveLength(1);
    expect(result.current[0].type).toBe('payout');
  });

  it('filters by date range', () => {
    const filters: TimelineFilters = {
      groupIds: [],
      dateRange: { start: new Date('2024-01-01'), end: new Date('2024-03-31') },
      eventTypes: [],
    };
    const { result } = renderHook(() => useFilteredContributions(mockData, filters));
    expect(result.current).toHaveLength(1);
    expect(result.current[0].id).toBe('1');
  });

  it('sorts results chronologically', () => {
    const unsorted = [mockData[2], mockData[0], mockData[1]];
    const filters: TimelineFilters = { groupIds: [], dateRange: { start: null, end: null }, eventTypes: [] };
    const { result } = renderHook(() => useFilteredContributions(unsorted, filters));
    expect(result.current.map((c) => c.id)).toEqual(['1', '2', '3']);
  });

  it('returns empty array when no matches', () => {
    const filters: TimelineFilters = { groupIds: ['g3'], dateRange: { start: null, end: null }, eventTypes: [] };
    const { result } = renderHook(() => useFilteredContributions(mockData, filters));
    expect(result.current).toHaveLength(0);
  });
});
