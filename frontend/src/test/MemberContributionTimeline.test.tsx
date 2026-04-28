import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { MemberContributionTimeline } from '../components/MemberContributionTimeline';
import { mockMemberContributions } from '../components/MemberContributionTimeline';

describe('MemberContributionTimeline', () => {
  it('renders timeline container', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} />);
    expect(screen.getByTestId('member-contribution-timeline')).toBeInTheDocument();
    expect(screen.getByTestId('timeline-svg')).toBeInTheDocument();
  });

  it('displays member name in title when provided', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} memberName="Alice" />);
    expect(screen.getByText("Alice's Contribution History")).toBeInTheDocument();
  });

  it('renders event nodes for contributions', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} />);
    expect(screen.getByTestId('timeline-node-c1')).toBeInTheDocument();
    expect(screen.getByTestId('timeline-node-c2')).toBeInTheDocument();
  });

  it('renders timeline axis', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} />);
    expect(screen.getByTestId('timeline-axis')).toBeInTheDocument();
  });

  it('shows event detail on node click', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} />);
    const node = screen.getByTestId('timeline-node-c1');
    fireEvent.click(node);
    expect(screen.getByTestId('event-detail')).toBeInTheDocument();
    expect(screen.getByText('Refactored auth hook')).toBeInTheDocument();
  });

  it('closes detail when close button clicked', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} />);
    fireEvent.click(screen.getByTestId('timeline-node-c1'));
    expect(screen.getByTestId('event-detail')).toBeInTheDocument();

    fireEvent.click(screen.getByTestId('close-detail'));
    expect(screen.queryByTestId('event-detail')).not.toBeInTheDocument();
  });

  it('renders filter panel', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} />);
    expect(screen.getByTestId('timeline-filters')).toBeInTheDocument();
  });

  it('filters by group when group chip clicked', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} />);
    const groupFilter = screen.getByTestId('group-filter-group-1');
    fireEvent.click(groupFilter);

    // After filtering to group-1, nodes from other groups should be gone
    expect(screen.queryByTestId('timeline-node-c2')).not.toBeInTheDocument();
    expect(screen.getByTestId('timeline-node-c1')).toBeInTheDocument();
  });

  it('filters by event type', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} />);
    const typeFilter = screen.getByTestId('type-filter-member_join');
    fireEvent.click(typeFilter);

    // Should only show member_join events
    expect(screen.getByTestId('timeline-node-c5')).toBeInTheDocument();
    expect(screen.queryByTestId('timeline-node-c1')).not.toBeInTheDocument();
  });

  it('filters by date range', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} />);
    const startInput = screen.getByTestId('date-start') as HTMLInputElement;
    const endInput = screen.getByTestId('date-end') as HTMLInputElement;

    fireEvent.change(startInput, { target: { value: '2025-01-01' } });
    fireEvent.change(endInput, { target: { value: '2025-12-31' } });

    // Most recent events should still be visible
    expect(screen.getByTestId('timeline-node-c15')).toBeInTheDocument();
  });

  it('renders reset zoom button', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} />);
    expect(screen.getByTestId('reset-zoom')).toBeInTheDocument();
  });

  it('clears filters when clear button clicked', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} />);
    fireEvent.click(screen.getByTestId('group-filter-group-1'));

    // Clear filters
    const clearBtn = screen.getByTestId('clear-filters');
    fireEvent.click(clearBtn);

    // All nodes should be back
    expect(screen.getByTestId('timeline-node-c2')).toBeInTheDocument();
    expect(screen.getByTestId('timeline-node-c1')).toBeInTheDocument();
  });

  it('handles empty contributions gracefully', () => {
    render(<MemberContributionTimeline contributions={[]} />);
    expect(screen.getByTestId('member-contribution-timeline')).toBeInTheDocument();
    expect(screen.queryByTestId('timeline-node-c1')).not.toBeInTheDocument();
  });

  it('displays subtitle with correct event and group counts', () => {
    render(<MemberContributionTimeline contributions={mockMemberContributions} />);
    expect(screen.getByText(/15 events across 4 groups/)).toBeInTheDocument();
  });
});
