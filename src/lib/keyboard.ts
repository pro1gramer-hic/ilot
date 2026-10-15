// The island never takes the keyboard by itself (it would steal it from the app the user
// types in). A click in one of its text fields is a deliberate gesture: the island's
// window then takes the keyboard, so that what the user types lands in the field.

/** A field the user types in: text inputs, text areas, editable content. */
export function isTextField(target: EventTarget | null): boolean {
  if (typeof HTMLElement === "undefined" || !(target instanceof HTMLElement)) return false;
  if (target.isContentEditable || target instanceof HTMLTextAreaElement) return true;
  if (!(target instanceof HTMLInputElement)) return false;
  return ["text", "search", "email", "url", "password", "number", "tel", ""].includes(target.type);
}
