interface AvatarProps {
  src?: string
  alt: string
  size?: 'sm' | 'md' | 'lg'
  className?: string
}

const sizeClasses = {
  sm: 'h-6 w-6',
  md: 'h-9 w-9',
  lg: 'h-12 w-12',
}

export function Avatar({ src, alt, size = 'md', className = '' }: AvatarProps) {
  return (
    <img
      src={src}
      alt={alt}
      className={`shrink-0 rounded-full object-cover bg-surface ${sizeClasses[size]} ${className}`}
      loading="lazy"
    />
  )
}
