import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type {
  AnimeSourceId,
  CatalogFilters,
  CatalogItem,
  CatalogPage,
  CatalogSort,
  CatalogType,
  CategoryInfo,
  MediaFilter,
} from "../types";
import { useDebouncedValue } from "../hooks/useDebouncedValue";
import { sourceSupportsFilmes } from "../utils/source";
import { Icon } from "./Icon";
import { PosterImage } from "./PosterImage";
import { AnimeAnalyzeModal } from "./AnimeAnalyzeModal";
import { SourcePicker } from "./SourcePicker";

interface CatalogGridProps {
  onSelectAnime: (url: string) => void;
  onDownloadStarted?: () => void;
}

export function CatalogGrid({ onSelectAnime, onDownloadStarted }: CatalogGridProps) {
  const [source, setSource] = useState<AnimeSourceId>("sushianimes");
  const [tab, setTab] = useState<CatalogType>("animes");
  const [page, setPage] = useState(1);
  const [data, setData] = useState<CatalogPage | null>(null);
  const [categories, setCategories] = useState<CategoryInfo[]>([]);
  const [categorySlug, setCategorySlug] = useState("");
  const [query, setQuery] = useState("");
  const [searchMode, setSearchMode] = useState(false);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [history, setHistory] = useState<string[]>([]);
  const [mediaFilter, setMediaFilter] = useState<MediaFilter>("all");
  const [sort, setSort] = useState<CatalogSort>("default");
  const [categoryFilter, setCategoryFilter] = useState("");
  const [titleFilter, setTitleFilter] = useState("");
  const [analyzeItem, setAnalyzeItem] = useState<CatalogItem | null>(null);

  const debouncedQuery = useDebouncedValue(query, 400);
  const debouncedTitleFilter = useDebouncedValue(titleFilter, 250);

  const filters = useMemo<CatalogFilters>(
    () => ({
      mediaFilter: searchMode ? mediaFilter : tab === "filmes" ? "filme" : tab === "animes" ? "anime" : mediaFilter,
      sort,
      category: categoryFilter.trim() || null,
      titleFilter: debouncedTitleFilter.trim() || null,
    }),
    [mediaFilter, sort, categoryFilter, debouncedTitleFilter, searchMode, tab]
  );

  useEffect(() => {
    if (searchMode) {
      setPage(1);
    }
  }, [debouncedQuery, searchMode]);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      let result: CatalogPage;
      if (searchMode && debouncedQuery.trim()) {
        result = await invoke<CatalogPage>("search_catalog", {
          req: {
            query: debouncedQuery.trim(),
            page,
            filters,
            source,
          },
        });
      } else {
        result = await invoke<CatalogPage>("browse_catalog", {
          req: {
            catalogType: tab,
            page,
            categorySlug: tab === "category" ? categorySlug || "acao" : null,
            filters,
            source,
          },
        });
      }
      setData(result);
    } catch (err) {
      setData({ items: [], page, hasNext: false });
      setLoadError(String(err));
    } finally {
      setLoading(false);
    }
  }, [tab, page, categorySlug, debouncedQuery, searchMode, filters, source]);

  useEffect(() => {
    invoke<CategoryInfo[]>("get_categories", { source }).then(setCategories).catch(() => {});
    invoke<string[]>("get_search_history").then(setHistory).catch(() => {});
  }, [source]);

  useEffect(() => {
    if (!sourceSupportsFilmes(source) && tab === "filmes") {
      setTab("animes");
      setPage(1);
    }
  }, [source, tab]);

  useEffect(() => {
    setData(null);
    setPage(1);
    setSearchMode(false);
    setQuery("");
  }, [source]);

  useEffect(() => {
    load();
  }, [load]);

  const startSearch = () => {
    if (!query.trim()) return;
    setSearchMode(true);
    setPage(1);
  };

  const clearSearch = () => {
    setSearchMode(false);
    setQuery("");
    setPage(1);
  };

  const resetFilters = () => {
    setMediaFilter("all");
    setSort("default");
    setCategoryFilter("");
    setTitleFilter("");
    setPage(1);
  };

  const resultCount = data?.items.length ?? 0;
  const hasActiveFilters =
    mediaFilter !== "all" ||
    sort !== "default" ||
    categoryFilter.trim() !== "" ||
    titleFilter.trim() !== "";

  return (
    <div className="catalog-panel">
      <div className="catalog-header">
        <h2>
          <Icon name="fa-clapperboard" /> Catálogo
        </h2>

        <div className="search-row">
          <div className="input-with-icon">
            <Icon name="fa-magnifying-glass" />
            <input
              type="search"
              placeholder="Buscar anime ou filme..."
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && startSearch()}
            />
          </div>
          <button type="button" className="btn-primary" onClick={startSearch}>
            <Icon name="fa-search" /> Buscar
          </button>
          {searchMode && (
            <button type="button" className="btn-ghost" onClick={clearSearch}>
              <Icon name="fa-xmark" /> Limpar
            </button>
          )}
        </div>

        {history.length > 0 && !searchMode && (
          <div className="search-history">
            <Icon name="fa-clock-rotate-left" /> Recentes:
            {history.map((h) => (
              <button
                key={h}
                type="button"
                className="chip"
                onClick={() => {
                  setQuery(h);
                  setSearchMode(true);
                  setPage(1);
                }}
              >
                {h}
              </button>
            ))}
          </div>
        )}
      </div>

      <SourcePicker
        value={source}
        onChange={(next) => {
          setSource(next);
          setPage(1);
          setSearchMode(false);
          setCategorySlug("");
        }}
      />

      <div className="catalog-filters">
        <div className="filter-group">
          <label>
            <Icon name="fa-filter" /> Tipo
          </label>
          <div className="filter-chips">
            {(
              [
                ["all", "Tudo", "fa-border-all"],
                ["anime", "Animes", "fa-tv"],
                ["filme", "Filmes", "fa-film"],
              ] as const
            ).map(([value, label, icon]) => (
              <button
                key={value}
                type="button"
                className={`filter-chip ${mediaFilter === value ? "active" : ""}`}
                onClick={() => {
                  setMediaFilter(value);
                  setPage(1);
                }}
              >
                <Icon name={icon} /> {label}
              </button>
            ))}
          </div>
        </div>

        <div className="filter-group">
          <label>
            <Icon name="fa-arrow-down-a-z" /> Ordenar
          </label>
          <select
            value={sort}
            onChange={(e) => {
              setSort(e.target.value as CatalogSort);
              setPage(1);
            }}
          >
            <option value="default">Padrão do site</option>
            <option value="titleAsc">Título A → Z</option>
            <option value="titleDesc">Título Z → A</option>
          </select>
        </div>

        <div className="filter-group">
          <label>
            <Icon name="fa-tag" /> Categoria
          </label>
          <select
            value={categoryFilter}
            onChange={(e) => {
              setCategoryFilter(e.target.value);
              setPage(1);
            }}
          >
            <option value="">Todas</option>
            {categories.map((c) => (
              <option key={c.slug} value={c.name}>
                {c.name}
              </option>
            ))}
          </select>
        </div>

        <div className="filter-group filter-grow">
          <label>
            <Icon name="fa-sliders" /> Refinar lista
          </label>
          <div className="input-with-icon">
            <Icon name="fa-filter" />
            <input
              type="search"
              placeholder="Filtrar por título na página..."
              value={titleFilter}
              onChange={(e) => {
                setTitleFilter(e.target.value);
                setPage(1);
              }}
            />
          </div>
        </div>

        {hasActiveFilters && (
          <button type="button" className="btn-ghost btn-sm filter-reset" onClick={resetFilters}>
            <Icon name="fa-rotate-left" /> Resetar filtros
          </button>
        )}
      </div>

      {!searchMode && (
        <div className="catalog-tabs">
          <button
            type="button"
            className={tab === "animes" ? "active" : ""}
            onClick={() => {
              setTab("animes");
              setPage(1);
            }}
          >
            <Icon name="fa-tv" /> Animes
          </button>
          <button
            type="button"
            className={tab === "filmes" ? "active" : ""}
            onClick={() => {
              setTab("filmes");
              setPage(1);
            }}
            style={sourceSupportsFilmes(source) ? undefined : { display: "none" }}
          >
            <Icon name="fa-film" /> Filmes
          </button>
          <button
            type="button"
            className={tab === "category" ? "active" : ""}
            onClick={() => {
              setTab("category");
              setPage(1);
            }}
          >
            <Icon name="fa-tags" /> Categorias
          </button>
          {tab === "category" && (
            <select
              value={categorySlug}
              onChange={(e) => {
                setCategorySlug(e.target.value);
                setPage(1);
              }}
            >
              <option value="">Selecione...</option>
              {categories.map((c) => (
                <option key={c.slug} value={c.slug}>
                  {c.name}
                </option>
              ))}
            </select>
          )}
        </div>
      )}

      <div className="catalog-results-bar">
        {searchMode ? (
          <span>
            <Icon name="fa-search" /> Resultados para <strong>{debouncedQuery}</strong>
          </span>
        ) : (
          <span>
            <Icon name="fa-list" /> Página {page}
          </span>
        )}
        <span className="result-count">
          {loading ? "Carregando..." : `${resultCount} item(ns)`}
        </span>
      </div>

      {loading && (
        <div className="loading">
          <Icon name="fa-spinner" spin /> Carregando catálogo...
        </div>
      )}

      <div className="catalog-grid">
        {data?.items.map((item) => (
          <CatalogCard
            key={item.url}
            item={item}
            onSelect={onSelectAnime}
            onAnalyze={() => setAnalyzeItem(item)}
          />
        ))}
      </div>

      {analyzeItem && (
        <AnimeAnalyzeModal
          item={analyzeItem}
          onClose={() => setAnalyzeItem(null)}
          onDownloadStarted={onDownloadStarted}
        />
      )}

      {loadError && (
        <p className="empty-state catalog-error">
          <Icon name="fa-triangle-exclamation" /> {loadError}
        </p>
      )}

      {!loading && !loadError && data?.items.length === 0 && (
        <p className="empty-state">
          <Icon name="fa-face-frown" /> Nenhum resultado encontrado. Tente outros filtros ou termos.
        </p>
      )}

      <div className="pagination">
        <button
          type="button"
          className="btn-ghost"
          disabled={page <= 1}
          onClick={() => setPage((p) => Math.max(1, p - 1))}
        >
          <Icon name="fa-chevron-left" /> Anterior
        </button>
        <span>
          Página <strong>{page}</strong>
        </span>
        <button
          type="button"
          className="btn-ghost"
          disabled={!data?.hasNext}
          onClick={() => setPage((p) => p + 1)}
        >
          Próxima <Icon name="fa-chevron-right" />
        </button>
      </div>
    </div>
  );
}

function CatalogCard({
  item,
  onSelect,
  onAnalyze,
}: {
  item: CatalogItem;
  onSelect: (url: string) => void;
  onAnalyze: () => void;
}) {
  const isMovie =
    item.category?.toLowerCase() === "filme" ||
    item.url.includes("/filme/") ||
    item.url.includes("/assistir/");

  return (
    <div className="catalog-card">
      <button type="button" className="catalog-card-main" onClick={() => onSelect(item.url)}>
        <div className="catalog-thumb">
          <PosterImage src={item.poster} alt={item.title} />
          {isMovie && (
            <span className="catalog-badge">
              <Icon name="fa-film" /> Filme
            </span>
          )}
        </div>
        <div className="catalog-info">
          <strong>{item.title}</strong>
          {item.category && (
            <small>
              <Icon name="fa-tag" /> {item.category}
            </small>
          )}
        </div>
      </button>
      <button
        type="button"
        className="catalog-analyze-btn"
        onClick={(e) => {
          e.stopPropagation();
          onAnalyze();
        }}
        title="Analisar e baixar"
      >
        <Icon name="fa-bolt" /> Analisar
      </button>
    </div>
  );
}
