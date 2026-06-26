import emblem from "@/assets/brand/mindflow-emblem.png";

interface Props {
  width?: number | string;
  className?: string;
}

export default function FlowMark({ width = 24, className }: Props) {
  return (
    <img
      src={emblem}
      alt=""
      width={width}
      className={className}
      draggable={false}
    />
  );
}
