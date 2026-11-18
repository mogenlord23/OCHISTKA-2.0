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

/** Превью как base64: если оригинала нет, ядро кладёт его файлом и ставит обоями. */
export async function thumbBase64(url: string): Promise<string | undefined> {
  try {
    const bytes = new Uint8Array(await (await fetch(url)).arrayBuffer());
    let binary = '';
    for (let i = 0; i < bytes.length; i += 0x8000) {
      binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    }
    return btoa(binary);
  } catch {
    return undefined;
  }
}
