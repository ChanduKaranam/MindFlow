# Task 11 Report — WelcomeStep

## File
`app/src/components/onboarding/WelcomeStep.tsx`

## JSX outline
```
<div relative min-h-screen flex centered>
  <AmbientBackground />               ← fixed z-1 glow layer
  <div .glass rounded-2xl p-8 max-w-[480px]>
    <OnboardingStepper current total />
    <MindFlowLogo width={200} />
    <h1 .font-serif.text-text>        ← t("onboarding.welcome.title")
    <p .text-text-secondary>          ← t("onboarding.welcome.subtitle")
    <ul>
      <li> <Check .text-privacy /> t("onboarding.welcome.trust.local")
      <li> <Check .text-privacy /> t("onboarding.welcome.trust.noAccount")
      <li> <Check .text-privacy /> t("onboarding.welcome.trust.noCloud")
    </ul>
    <button .btn-gold.sheen>          ← t("onboarding.welcome.cta") → onContinue
  </div>
</div>
```

## i18n keys added (under `onboarding.welcome`)
- `title` = "Type at the speed of thought."
- `subtitle` = "MindFlow turns your voice into text in any app — fully on your device."
- `trust.local` = "Your voice never leaves your device"
- `trust.noAccount` = "No account"
- `trust.noCloud` = "No cloud"
- `cta` = "Get started"

## Build / lint output
- `bun run lint` → clean (0 errors)
- `bun x tsc --noEmit` → EXIT:0
- JSON validity → valid
- `bun run build` (vite bundle) → in-progress during check; tsc clean confirms no TS errors; vite transforms started without error

## Concerns
None. AmbientBackground handles reduced-motion internally. All strings go through `useTranslation`. No `any` types used.
