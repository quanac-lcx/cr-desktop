import { TextField, styled } from "@mui/material";

export const FilledTextField = styled(TextField)(() => ({
  "& .Mui-disabled:before": {
    border: "none",
  },
}));

export const DenseFilledTextField = styled(FilledTextField)(({ theme }) => ({
  "& .MuiOutlinedInput-input": {
    paddingTop: theme.spacing(1.2),
    paddingBottom: theme.spacing(1.2),
    fontSize: theme.typography.body2.fontSize,
  },
  "& .MuiInputBase-root.MuiOutlinedInput-root": {
    paddingTop: 0,
    paddingBottom: 0,
    fontSize: theme.typography.body2.fontSize,
  },
  "& .MuiInputLabel-root": {
    fontSize: theme.typography.body2.fontSize,
    // no class .Mui-focused
    "&:not(.Mui-focused):not(.MuiInputLabel-shrink)": {
      transform: "translate(14px, 10px) scale(1)",
    },
  },
}));
