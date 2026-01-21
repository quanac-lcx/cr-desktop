import { Box, Typography } from "@mui/material";
import { useTranslation } from "react-i18next";

export default function GeneralSection() {
  const { t } = useTranslation();

  return (
    <Box>
      <Typography variant="h5" fontWeight={500} gutterBottom>
        {t("settings.general")}
      </Typography>
      <Typography variant="body2" color="text.secondary">
        {t("settings.generalComingSoon")}
      </Typography>
    </Box>
  );
}
