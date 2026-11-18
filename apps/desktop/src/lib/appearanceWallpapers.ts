import gallery from '../assets/wallpapers/gallery.json';

/** Фото из коллекции владельца: превью лежат в assets/wallpapers/thumbs, оригиналы не вшиваются. */
export interface WallpaperPhoto {
  id: string;
  title: string;
  thumb: string;
  width: number;
  height: number;
}

const THUMB_URLS = import.meta.glob('../assets/wallpapers/thumbs/*.jpg', {
  eager: true,
  query: '?url',
  import: 'default',
}) as Record<string, string>;

function thumbUrl(file: string): string {
  return THUMB_URLS[`../assets/wallpapers/thumbs/${file}`] ?? '';
}

export const WALLPAPER_PHOTOS: readonly WallpaperPhoto[] = (
  gallery as { id: string; title: string; thumb: string; width: number; height: number }[]
).map((item) => ({
  id: item.id,
  title: item.title,
  thumb: thumbUrl(item.thumb),
  width: item.width,
  height: item.height,
}));
