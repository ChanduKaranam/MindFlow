import { useTranslation } from "react-i18next";
import { Check } from "lucide-react";
import AmbientBackground from "@/components/shared/AmbientBackground";
import MindFlowLogo from "@/components/icons/MindFlowLogo";
import OnboardingStepper from "./OnboardingStepper";

interface Props {
  onContinue: () => void;
  stepIndex: number;
  stepTotal: number;
}

export default function WelcomeStep({ onContinue, stepIndex, stepTotal }: Props) {
  const { t } = useTranslation();

  return (
    <div className="relative min-h-screen flex items-center justify-center p-6">
      <AmbientBackground />

      <div
        className="glass rounded-2xl p-8 w-full flex flex-col gap-6"
        style={{ maxWidth: "480px" }}
      >
        <OnboardingStepper current={stepIndex} total={stepTotal} />

        <div className="flex justify-center">
          <MindFlowLogo width={200} />
        </div>

        <div className="flex flex-col gap-2 text-center">
          <h1 className="font-serif text-3xl font-light text-text leading-tight">
            {t("onboarding.welcome.title")}
          </h1>
          <p className="text-text-secondary text-base">
            {t("onboarding.welcome.subtitle")}
          </p>
        </div>

        <ul className="flex flex-col gap-3" aria-label={t("onboarding.welcome.title")}>
          <li className="flex items-center gap-3">
            <Check
              size={18}
              className="text-privacy shrink-0"
              aria-hidden="true"
            />
            <span className="text-text-secondary text-sm">
              {t("onboarding.welcome.trust.local")}
            </span>
          </li>
          <li className="flex items-center gap-3">
            <Check
              size={18}
              className="text-privacy shrink-0"
              aria-hidden="true"
            />
            <span className="text-text-secondary text-sm">
              {t("onboarding.welcome.trust.noAccount")}
            </span>
          </li>
          <li className="flex items-center gap-3">
            <Check
              size={18}
              className="text-privacy shrink-0"
              aria-hidden="true"
            />
            <span className="text-text-secondary text-sm">
              {t("onboarding.welcome.trust.noCloud")}
            </span>
          </li>
        </ul>

        <button
          type="button"
          onClick={onContinue}
          className="btn-gold sheen w-full rounded-lg px-6 py-3 text-base font-semibold cursor-pointer"
        >
          {t("onboarding.welcome.cta")}
        </button>
      </div>
    </div>
  );
}
