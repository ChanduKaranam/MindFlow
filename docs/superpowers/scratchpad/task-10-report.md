# Task 10 Report: OnboardingStepper

## Status
DONE

## Files created / modified
- **Created:** `app/src/components/onboarding/OnboardingStepper.tsx`
- **Modified:** `app/src/i18n/locales/en/translation.json` — added `onboarding.stepper.label`
- **Modified:** `app/src/components/onboarding/index.ts` — exported `OnboardingStepper`

## JSX (final)
```tsx
interface Props { current: number; total: number; className?: string; }
```
- Horizontal row of `total` small rounded bars (`h-1 flex-1 rounded-full`).
- Segments with index `< current` (done) and `=== current` (active) → `bg-accent` (gold).
- Upcoming segments → `bg-border`.
- Transition: `transition-[background-color] duration-150 ease-out motion-reduce:transition-none`.
- Label rendered via `t("onboarding.stepper.label", { current, total })`, styled `text-text-secondary text-sm`.
- Container: `role="progressbar"`, `aria-valuemin={1}`, `aria-valuemax={total}`, `aria-valuenow={current}`, `aria-label={label}`.

## i18n key added
```json
"stepper": { "label": "Step {{current}} of {{total}}" }
```
Added under `onboarding` object in `en/translation.json`. JSON validated with Python `json.load`.

## Build / lint output
- `bun run build` (tsc + vite): ✓ exit 0, built in 5m 48s
- `bun run lint` (eslint src): ✓ exit 0, no warnings

## Concerns
None. The large-chunk warning (index.js > 500 kB) is pre-existing, unrelated to this change.
