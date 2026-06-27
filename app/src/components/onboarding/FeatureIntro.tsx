import { useTranslation } from "react-i18next";
import { Headphones, MessageSquare, BookText, Waves } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import AmbientBackground from "@/components/shared/AmbientBackground";
import OnboardingStepper from "./OnboardingStepper";

interface Props {
  onFinish: () => void;
  stepIndex: number;
  stepTotal: number;
}

interface FeatureCardProps {
  Icon: LucideIcon;
  title: string;
  desc: string;
}

function FeatureCard({ Icon, title, desc }: FeatureCardProps) {
  return (
    <li className="flex items-start gap-3">
      <Icon
        size={20}
        className="text-accent shrink-0 mt-0.5"
        aria-hidden="true"
      />
      <div className="flex flex-col gap-0.5">
        <span className="text-text text-sm font-semibold">{title}</span>
        <span className="text-text-secondary text-sm">{desc}</span>
      </div>
    </li>
  );
}

export default function FeatureIntro({ onFinish, stepIndex, stepTotal }: Props) {
  const { t } = useTranslation();

  return (
    <div className="relative min-h-screen flex items-center justify-center p-6">
      <AmbientBackground />
      <div
        className="glass rounded-2xl p-8 w-full flex flex-col gap-6"
        style={{ maxWidth: "480px" }}
      >
        <OnboardingStepper current={stepIndex} total={stepTotal} />

        <h1 className="font-serif text-2xl font-light text-text leading-tight">
          {t("onboarding.features.title")}
        </h1>

        <ul className="flex flex-col gap-5" aria-label={t("onboarding.features.title")}>
          <FeatureCard
            Icon={Headphones}
            title={t("onboarding.features.handsFree.title")}
            desc={t("onboarding.features.handsFree.desc")}
          />
          <FeatureCard
            Icon={MessageSquare}
            title={t("onboarding.features.spokenCommands.title")}
            desc={t("onboarding.features.spokenCommands.desc")}
          />
          <FeatureCard
            Icon={BookText}
            title={t("onboarding.features.dictionary.title")}
            desc={t("onboarding.features.dictionary.desc")}
          />
          <FeatureCard
            Icon={Waves}
            title={t("onboarding.features.noiseSuppression.title")}
            desc={t("onboarding.features.noiseSuppression.desc")}
          />
        </ul>

        <button
          type="button"
          onClick={onFinish}
          className="btn-gold sheen w-full rounded-lg px-6 py-3 text-base font-semibold cursor-pointer"
        >
          {t("onboarding.features.cta")}
        </button>
      </div>
    </div>
  );
}
