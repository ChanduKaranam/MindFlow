import { useTranslation } from "react-i18next";

interface Props {
  current: number;
  total: number;
  className?: string;
}

export default function OnboardingStepper({
  current,
  total,
  className = "",
}: Props) {
  const { t } = useTranslation();
  const label = t("onboarding.stepper.label", { current, total });

  return (
    <div className={`flex flex-col gap-2 ${className}`}>
      <div
        role="progressbar"
        aria-valuemin={1}
        aria-valuemax={total}
        aria-valuenow={current}
        aria-label={label}
        className="flex gap-1.5"
      >
        {Array.from({ length: total }, (_, i) => {
          const filled = i < current;
          return (
            <div
              key={i}
              className={[
                "h-1 flex-1 rounded-full",
                "transition-[background-color] duration-150 ease-out",
                "motion-reduce:transition-none",
                filled ? "bg-accent" : "bg-border",
              ].join(" ")}
            />
          );
        })}
      </div>
      <p className="text-text-secondary text-sm">{label}</p>
    </div>
  );
}
