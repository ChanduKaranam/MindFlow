import logo from "@/assets/brand/mindflow-logo.png";
import emblem from "@/assets/brand/mindflow-emblem.png";

interface Props {
  width?: number;
  className?: string;
  emblemOnly?: boolean;
}

export default function MindFlowLogo({ width = 200, className, emblemOnly }: Props) {
  return (
    <img
      src={emblemOnly ? emblem : logo}
      alt="MindFlow"
      width={width}
      className={className}
      draggable={false}
    />
  );
}
