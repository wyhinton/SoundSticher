import { writable } from 'svelte/store';

// Store to track which dropdown is currently open
// Value can be null (no dropdown open) or a unique identifier for the open dropdown
export const openDropdown = writable<string | null>(null);

// Function to open a dropdown and close any others
export function openDropdownExclusive(dropdownId: string) {
  openDropdown.set(dropdownId);
}

// Function to close a specific dropdown
export function closeDropdown(dropdownId: string) {
  openDropdown.update(current => {
    return current === dropdownId ? null : current;
  });
}

// Function to close all dropdowns
export function closeAllDropdowns() {
  openDropdown.set(null);
}
