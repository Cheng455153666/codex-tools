import { CSSProperties } from 'react';
import traeIcon from '../../assets/icons/trae.png';

type TraeIconProps = {
  className?: string;
  style?: CSSProperties;
};

export function TraeIcon({ className = 'nav-item-icon', style }: TraeIconProps) {
  return (
    <img
      className={className}
      style={style}
      src={traeIcon}
      alt=""
      aria-hidden="true"
      draggable={false}
    />
  );
}
