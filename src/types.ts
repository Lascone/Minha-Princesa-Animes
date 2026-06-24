export type AnimeSourceId =
  | "sushianimes"
  | "goyabu"
  | "meusanimes"
  | "animesonlinecc"
  | "animesdigital";

export type EpisodeInfo = {
  number: number;
  season: number;
  title: string;
  description?: string;
  url: string;
};

export type SeasonInfo = {
  number: number;
  episodes: EpisodeInfo[];
};

export type AnimeInfo = {
  title: string;
  url: string;
  poster?: string;
  synopsis?: string;
  seasons: SeasonInfo[];
};

export type CatalogItem = {
  title: string;
  url: string;
  poster?: string;
  category?: string;
};

export type CatalogPage = {
  items: CatalogItem[];
  page: number;
  hasNext: boolean;
};

export type CategoryInfo = {
  name: string;
  slug: string;
  url: string;
};

export type DownloadStatus =
  | "queued"
  | "downloading"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

export type DownloadItem = {
  id: string;
  animeTitle: string;
  episodeLabel: string;
  episode: EpisodeInfo;
  status: DownloadStatus;
  progress: number;
  speed: string;
  outputPath?: string;
  error?: string;
  posterUrl?: string;
  posterPath?: string;
  updatedAt?: number;
};

export type DownloadRequest = {
  animeTitle: string;
  episodes: EpisodeInfo[];
  posterUrl?: string;
  animeUrl?: string;
};

export type DownloadFilter = "all" | "active" | "queued" | "completed" | "failed";

export type AppSettings = {
  downloadFolder: string;
  namingTemplate: string;
  maxConcurrent: number;
  ffmpegPath: string;
  overwrite: boolean;
  theme: string;
  notifications?: boolean;
};

export type CatalogType = "animes" | "filmes" | "category";

export type MediaFilter = "all" | "anime" | "filme";

export type CatalogSort = "default" | "titleAsc" | "titleDesc";

export type CatalogFilters = {
  mediaFilter?: MediaFilter;
  sort?: CatalogSort;
  category?: string | null;
  titleFilter?: string | null;
};

export type SearchRequest = {
  query: string;
  page: number;
  filters?: CatalogFilters;
  source?: AnimeSourceId;
};

export type BrowseRequest = {
  catalogType: CatalogType;
  page: number;
  categorySlug?: string | null;
  filters?: CatalogFilters;
  source?: AnimeSourceId;
};

export type Page = "home" | "catalog" | "downloads" | "settings";
