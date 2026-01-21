import { Box, Typography, Link } from "@mui/material";
import { useTranslation } from "react-i18next";

export default function AboutSection() {
  const { t } = useTranslation();

  return (
    <Box>
      <Typography variant="h5" fontWeight={500} gutterBottom>
        {t("settings.about")}
      </Typography>
      <Typography variant="body2" color="text.secondary" paragraph>
        Cloudreve Desktop
      </Typography>
      <Typography variant="body2" color="text.secondary">
        <Link
          href="https://github.com/cloudreve/desktop"
          target="_blank"
          rel="noopener noreferrer"
        >
          GitHub
        </Link>
      </Typography>
    </Box>
  );
}
