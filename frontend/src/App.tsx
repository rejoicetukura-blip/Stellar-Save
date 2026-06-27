import { lazy, Suspense } from "react";
import { Box, CircularProgress } from "@mui/material";
import "./App.css";
import { useOfflineSyncInit } from "./hooks/useOfflineSync";

const AppRouter = lazy(() =>
  import("./routing/AppRouter").then((m) => ({ default: m.AppRouter }))
);

function RouteLoadingFallback() {
  return (
    <Box
      sx={{
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        minHeight: "100vh",
      }}
    >
      <CircularProgress />
    </Box>
  );
}

export default function App() {
  // Initialize offline sync service
  useOfflineSyncInit();

  return (
    <Suspense fallback={<RouteLoadingFallback />}>
      <AppRouter />
    </Suspense>
  );
}
