# Timeline Resizable Layout

This document explains the user-resizable timeline area feature and its integration with the application state.

## Overview

The timeline area can be resized by the user via a draggable divider, allowing them to adjust the balance between the main content area (file table, operations) and the timeline playback view. The timeline height is persisted in `appState.uiSettings.timelineHeight` for a consistent experience across sessions.

## Architecture

### State Management

**Location**: `src/lib/state/state.svelte.ts`

The timeline height is stored as a percentage (10-60) in `appState.uiSettings.timelineHeight`:

```typescript
export interface AppState {
  uiSettings?: {
    // ... other settings
    /** Timeline height as percentage of viewport (10-60) */
    timelineHeight?: number;
  };
}
```

**Default**: `30` (30% of viewport)

### Layout Implementation

**Location**: `src/lib/Greet.svelte`

The main layout uses viewport-based heights (`vh` units) to divide the screen:

```svelte
<!-- Content area takes remaining space -->
<div style:height="{100 - timelineHeight}vh" class="content-area">
  <!-- File table, operations, etc. -->
</div>

<!-- Draggable divider -->
<div class="timeline-divider" on:mousedown={handleDividerMouseDown}></div>

<!-- Timeline area -->
<div style:height="{timelineHeight}vh" class="timeline-container">
  <PlottedInfo></PlottedInfo>
  <Plotted></Plotted>
</div>
```

## Resizing Behavior

### User Interaction

1. **Hover**: The divider highlights (changes to primary color) to indicate it's interactive
2. **Drag**: User clicks and drags the divider vertically
3. **Constraints**: Timeline height is constrained between 10% and 60% of viewport
4. **Visual Feedback**: Cursor changes to `ns-resize` during drag

### Implementation Details

```typescript
// Reactive timeline height from appState
$: timelineHeight = $appState.uiSettings?.timelineHeight || 30;

// Update appState when timeline height changes
function setTimelineHeight(height: number) {
  appState.update(s => ({
    ...s,
    uiSettings: {
      ...s.uiSettings,
      timelineHeight: height,
    },
  }));
}

function handleDividerMouseMove(event: MouseEvent) {
  if (!isDraggingDivider) return;

  const viewportHeight = window.innerHeight;
  const mouseY = event.clientY;

  // Calculate new timeline height as percentage
  const newHeightPercent = ((viewportHeight - mouseY) / viewportHeight) * 100;

  // Constrain between 10% and 60%
  const constrainedHeight = Math.max(10, Math.min(60, newHeightPercent));
  setTimelineHeight(constrainedHeight);
}
```

### Event Listeners

The component registers global mouse event listeners for smooth dragging:

- `mousedown` on divider: Start drag
- `mousemove` on window: Update height during drag
- `mouseup` on window: End drag

These are cleaned up in `onDestroy` to prevent memory leaks.

## Divider Styling

The divider is designed to be minimal and unobtrusive:

```css
.timeline-divider {
  position: relative;
  height: 6px;
  background: var(--bs-border-color);
  cursor: ns-resize;
  transition: background-color 0.15s ease;
}

.timeline-divider:hover,
.timeline-divider:focus {
  background: var(--bs-primary);
}

.timeline-divider.dragging {
  background: var(--bs-primary);
}
```

**Key Features**:

- 6px height for easy targeting
- Thin, subtle appearance
- Hover and focus states for accessibility
- No visible handle (simplified from earlier design)
- Smooth color transitions

## Accessibility

The divider includes proper ARIA attributes:

```svelte
<div
  class="timeline-divider"
  role="separator"
  aria-orientation="horizontal"
  aria-label="Resize timeline"
  tabindex="0"
></div>
```

- `role="separator"`: Indicates semantic purpose
- `aria-orientation="horizontal"`: Describes divider direction
- `aria-label`: Provides screen reader description
- `tabindex="0"`: Allows keyboard focus

## Persistence

Timeline height is automatically persisted via the `svelte-persisted-store` used for `appState`:

1. User drags divider to new height
2. `setTimelineHeight()` updates `appState.uiSettings.timelineHeight`
3. `appState` is persisted to localStorage
4. On next app load, timeline restores to saved height

**Migration**: The `validateAndMigrateAppState` function ensures old states without `timelineHeight` get the default value of `30`.

## Integration with Scalable Timeline

The resizable layout works seamlessly with the scalable SVG timeline (see `timeline-scalable-layout.md`):

1. **ResizeObserver** in `Timeline.svelte` detects when the timeline container height changes
2. **Scalable content region** adapts to the new height automatically
3. **Waveforms** scale vertically to fill the available space
4. **Fixed elements** (header, axis) remain properly positioned

## Best Practices

### For Developers

1. **Always use the setter**: Use `setTimelineHeight()` instead of directly mutating `appState`
2. **Respect constraints**: Keep height between 10-60% for usability
3. **Clean up listeners**: Remember to remove event listeners in `onDestroy`
4. **Test responsiveness**: Verify behavior at different viewport sizes

### For Users

1. **Drag anywhere**: The entire 6px divider area is draggable
2. **Keyboard navigation**: Can be focused for accessibility
3. **Visual feedback**: Divider highlights on hover to show it's interactive
4. **Persistent**: Your preferred height is saved and restored

## Future Enhancements

Potential improvements to consider:

1. **Keyboard resize**: Add arrow key support for accessibility
2. **Double-click reset**: Double-click divider to return to default height
3. **Snap points**: Add subtle snap points at common heights (25%, 33%, 50%)
4. **Min content size**: Prevent collapsing panels below minimum usable height
5. **Mobile support**: Add touch event handling for mobile devices

## Related Documentation

- **Status System**: `status-system.md` - Centralized status management
- **Scalable Timeline**: `timeline-scalable-layout.md` - SVG layout and scaling
- **State Management**: `../src/lib/state/state.svelte.ts` - Application state structure

## Troubleshooting

### Timeline height not persisting

**Cause**: localStorage might be disabled or cleared  
**Solution**: Check browser settings, verify `appState` is properly configured

### Divider not draggable

**Cause**: Event listeners not attached or removed  
**Solution**: Check `onMount`/`onDestroy` lifecycle, verify no JavaScript errors

### Jerky resize performance

**Cause**: Heavy computation during resize  
**Solution**: Ensure `handleDividerMouseMove` is efficient, check Timeline ResizeObserver

### Height constraints not working

**Cause**: Incorrect constraint logic  
**Solution**: Verify `Math.max(10, Math.min(60, newHeightPercent))` is applied
