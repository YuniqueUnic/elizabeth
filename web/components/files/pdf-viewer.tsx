import { useState, useEffect } from "react";
import { Document, Page, pdfjs } from "react-pdf";
import "react-pdf/dist/Page/AnnotationLayer.css";
import "react-pdf/dist/Page/TextLayer.css";
import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight, ZoomIn, ZoomOut } from "lucide-react";
import { LoadingSpinner } from "@/components/ui/loading-spinner";
import { useTranslations } from "next-intl";
import { useSecureBlobUrl } from "@/hooks/use-secure-blob-url";

// Configure PDF.js worker
pdfjs.GlobalWorkerOptions.workerSrc =
  `//unpkg.com/pdfjs-dist@${pdfjs.version}/build/pdf.worker.min.mjs`;

interface PDFViewerProps {
  url: string;
  roomName?: string;
  className?: string;
}

export function PDFViewer({ url, roomName, className = "" }: PDFViewerProps) {
  const t = useTranslations("room.pdf");
  const [numPages, setNumPages] = useState<number>(0);
  const [pageNumber, setPageNumber] = useState<number>(1);
  const [scale, setScale] = useState<number>(1.0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const {
    resolvedSrc,
    loading: isHookLoading,
    error: hookError,
  } = useSecureBlobUrl(url, roomName);

  useEffect(() => {
    if (hookError) {
      console.error("Secure PDF load failed:", hookError);
      setError(t("loadFailed"));
      setLoading(false);
    }
  }, [hookError, t]);

  function onDocumentLoadSuccess({ numPages }: { numPages: number }) {
    setNumPages(numPages);
    setLoading(false);
  }

  function onDocumentLoadError(error: Error) {
    console.error("PDF load error:", error);
    setError(t("loadFailed"));
    setLoading(false);
  }

  const goToPrevPage = () => setPageNumber((page) => Math.max(page - 1, 1));
  const goToNextPage = () =>
    setPageNumber((page) => Math.min(page + 1, numPages));
  const handleZoomIn = () => setScale((s) => Math.min(s + 0.2, 3));
  const handleZoomOut = () => setScale((s) => Math.max(s - 0.2, 0.5));

  if (error) {
    return (
      <div className="flex items-center justify-center h-full p-8 text-destructive">
        <p>{error}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Toolbar */}
      <div className="flex items-center justify-between gap-2 p-2 border-b bg-muted/30">
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={goToPrevPage}
            disabled={pageNumber <= 1 || loading || isHookLoading}
            title={t("prevPage")}
          >
            <ChevronLeft className="h-4 w-4" />
          </Button>
          <span className="text-sm text-muted-foreground min-w-20 text-center">
            {loading || isHookLoading ? t("loading") : `${pageNumber} / ${numPages}`}
          </span>
          <Button
            variant="ghost"
            size="sm"
            onClick={goToNextPage}
            disabled={pageNumber >= numPages || loading || isHookLoading}
            title={t("nextPage")}
          >
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>

        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={handleZoomOut}
            disabled={scale <= 0.5 || loading || isHookLoading}
            title={t("zoomOut")}
          >
            <ZoomOut className="h-4 w-4" />
          </Button>
          <span className="text-sm text-muted-foreground min-w-[3rem] text-center">
            {Math.round(scale * 100)}%
          </span>
          <Button
            variant="ghost"
            size="sm"
            onClick={handleZoomIn}
            disabled={scale >= 3 || loading || isHookLoading}
            title={t("zoomIn")}
          >
            <ZoomIn className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* PDF Container */}
      <div className="flex-1 overflow-auto flex items-start justify-center p-4 bg-muted/10">
        {(loading || isHookLoading) && (
          <div className="flex items-center justify-center p-8">
            <LoadingSpinner className="h-8 w-8" />
          </div>
        )}
        {!isHookLoading && resolvedSrc && (
          <Document
            file={resolvedSrc}
            onLoadSuccess={onDocumentLoadSuccess}
            onLoadError={onDocumentLoadError}
            loading={null}
            className={className}
          >
            <Page
              pageNumber={pageNumber}
              scale={scale}
              renderTextLayer={true}
              renderAnnotationLayer={true}
            />
          </Document>
        )}
      </div>
    </div>
  );
}
