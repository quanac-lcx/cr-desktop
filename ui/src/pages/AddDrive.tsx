import {
  Box,
  Button,
  Container,
  Snackbar,
  Typography,
} from "@mui/material";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import defaultLogo from "../assets/cloudreve.svg";
import { FilledTextField } from "../common/StyledComponent";
import { fetch } from "@tauri-apps/plugin-http";

const MIN_VERSION = "4.12.0";

interface PingResponse {
  code: number;
  data: string;
  msg: string;
}

type ValidationErrorType = "httpError" | "apiError" | "versionTooLow" | "connectionFailed";

interface ValidationError {
  type: ValidationErrorType;
  params: Record<string, string>;
}

/**
 * Compare two semver version strings
 * Returns: -1 if a < b, 0 if a == b, 1 if a > b
 */
function compareSemver(a: string, b: string): number {
  const partsA = a.split(".").map(Number);
  const partsB = b.split(".").map(Number);

  for (let i = 0; i < Math.max(partsA.length, partsB.length); i++) {
    const numA = partsA[i] || 0;
    const numB = partsB[i] || 0;
    if (numA < numB) return -1;
    if (numA > numB) return 1;
  }
  return 0;
}

/**
 * Validate site version by pinging the API endpoint
 * @param siteUrl - The base URL of the Cloudreve site
 * @returns The version string if valid, throws a ValidationError otherwise
 */
async function validateSiteVersion(siteUrl: string): Promise<string> {
  let response: Response;
  try {
    const url = new URL("/api/v4/site/ping", siteUrl);
    response = await fetch(url.toString());
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    throw { type: "connectionFailed", params: { message } } as ValidationError;
  }

  if (!response.ok) {
    throw { type: "httpError", params: { status: String(response.status) } } as ValidationError;
  }

  const data: PingResponse = await response.json();
  if (data.code !== 0) {
    throw { type: "apiError", params: { message: data.msg || "Unknown error" } } as ValidationError;
  }

  // Remove -pro suffix if present
  const version = data.data.replace(/-pro$/, "");

  // Check if version is >= MIN_VERSION
  if (compareSemver(version, MIN_VERSION) < 0) {
    throw { type: "versionTooLow", params: { version, minVersion: MIN_VERSION } } as ValidationError;
  }

  return version;
}

function isValidationError(error: unknown): error is ValidationError {
  return (
    typeof error === "object" &&
    error !== null &&
    "type" in error &&
    "params" in error
  );
}

interface ManifestIcon {
  sizes: string;
  src: string;
  type?: string;
}

interface Manifest {
  icons?: ManifestIcon[];
}

/**
 * Check if a string is a valid URL
 */
function isValidUrl(url: string): boolean {
  try {
    new URL(url);
    return true;
  } catch {
    return false;
  }
}

/**
 * Fetch manifest.json and find the largest icon (preferably 512x512)
 */
async function fetchSiteIcon(siteUrl: string): Promise<string | null> {
  const manifestUrl = new URL("/manifest.json", siteUrl);
  const response = await fetch(manifestUrl.toString());

  if (!response.ok) {
    throw new Error(`Failed to fetch manifest: ${response.status}`);
  }

  const manifest: Manifest = await response.json();

  if (!manifest.icons || manifest.icons.length === 0) {
    return null;
  }

  // Find the largest icon, preferring 512x512
  let bestIcon: ManifestIcon | null = null;
  let bestSize = 0;

  for (const icon of manifest.icons) {
    // Parse sizes like "512x512" or "64x64 32x32 24x24 16x16"
    const sizeMatches = icon.sizes.match(/(\d+)x(\d+)/g);
    if (sizeMatches) {
      for (const sizeStr of sizeMatches) {
        const [width] = sizeStr.split("x").map(Number);
        if (width > bestSize) {
          bestSize = width;
          bestIcon = icon;
        }
      }
    }
  }

  if (!bestIcon) {
    return null;
  }

  // Resolve the icon URL relative to the site URL
  const iconUrl = new URL(bestIcon.src, siteUrl);
  return iconUrl.toString();
}

export default function AddDrive() {
  const { t } = useTranslation();
  const [siteUrl, setSiteUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [snackbarOpen, setSnackbarOpen] = useState(false);
  const [logo, setLogo] = useState(defaultLogo);
  const lastFetchedUrl = useRef<string>("");

  // Fetch site icon when URL changes and is valid
  const handleUrlBlur = () => {
    const trimmedUrl = siteUrl.trim();
    if (!isValidUrl(trimmedUrl) || trimmedUrl === lastFetchedUrl.current) {
      return;
    }

    lastFetchedUrl.current = trimmedUrl;

    fetchSiteIcon(trimmedUrl)
      .then((iconUrl) => {
        if (iconUrl) {
          // Preload the image to ensure it loads successfully
          const img = new Image();
          img.onload = () => {
            setLogo(iconUrl);
          };
          img.onerror = () => {
            console.error("Failed to load site icon:", iconUrl);
          };
          img.src = iconUrl;
        }
      })
      .catch((err) => {
        console.error("Failed to fetch manifest:", err);
      });
  };

  // Reset logo when URL is cleared or becomes invalid
  useEffect(() => {
    if (!siteUrl.trim() || !isValidUrl(siteUrl.trim())) {
      setLogo(defaultLogo);
      lastFetchedUrl.current = "";
    }
  }, [siteUrl]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setSnackbarOpen(false);
    try {
      const version = await validateSiteVersion(siteUrl);
      console.log("Site version:", version);
      // TODO: Continue with drive setup
    } catch (error) {
      if (isValidationError(error)) {
        setError(t(`addDrive.errors.${error.type}`, error.params));
      } else {
        const message = error instanceof Error ? error.message : String(error);
        setError(t("addDrive.errors.connectionFailed", { message }));
      }
      setSnackbarOpen(true);
    } finally {
      setLoading(false);
    }
  };

  const handleCloseSnackbar = () => {
    setSnackbarOpen(false);
  };

  return (
    <Container maxWidth="sm">
      <Box
        sx={{
          minHeight: "100vh",
          display: "flex",
          flexDirection: "column",
          justifyContent: "center",
          alignItems: "center",
          py: 4,
        }}
      >
        <Box
          sx={{
            p: 4,
            width: "100%",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: 1,
            borderRadius: 3,
          }}
        >
          <Box
            component="img"
            src={logo}
            alt="Cloudreve"
            sx={{
              width: 120,
              height: "auto",
              mb: 2,
            }}
          />

          <Typography sx={{mt:2}} variant="h5" component="h1" fontWeight={500}>
            {t("addDrive.title")}
          </Typography>

          <Typography variant="body2" color="text.secondary" textAlign="center">
            {t("addDrive.description")}
          </Typography>

          <Box
            component="form"
            onSubmit={handleSubmit}
            sx={{
              width: "100%",
              display: "flex",
              flexDirection: "column",
              gap: 2,
              mt:2,
            }}
          >
            <FilledTextField
              fullWidth
              autoComplete="off"
              slotProps={{
                input:{
                  readOnly:loading,
                }
              }}
              label={t("addDrive.siteUrl")}
              placeholder={t("addDrive.siteUrlPlaceholder")}
              value={siteUrl}
              onChange={(e) => setSiteUrl(e.target.value)}
              onBlur={handleUrlBlur}
              variant="filled"
              type="url"
              required
            />

            <Button
              type="submit"
              variant="contained"
              size="large"
              loading={loading}
              fullWidth
            >
              {t("addDrive.connect")}
            </Button>
          </Box>
        </Box>
      </Box>
      <Snackbar
        open={snackbarOpen}
        autoHideDuration={6000}
        onClose={handleCloseSnackbar}
        message={error}
      />
    </Container>
  );
}
