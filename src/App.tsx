import { HashRouter, Route } from "@solidjs/router";
import { AppShell } from "./components/AppShell";
import { AddDevice } from "./routes/AddDevice";
import { Home } from "./routes/Home";
import { Settings } from "./routes/Settings";
import { DeviceLayout } from "./routes/device/DeviceLayout";
import { Remote } from "./routes/device/Remote";
import { Apps } from "./routes/device/Apps";
import { Install } from "./routes/device/Install";
import { Overview, Themes } from "./routes/device/sections";

// Hash routing: the app is served from Tauri's asset protocol, where deep paths
// would not resolve on reload; hash URLs always load index.html.
function App() {
  return (
    <HashRouter root={AppShell}>
      <Route path="/" component={Home} />
      <Route path="/add" component={AddDevice} />
      <Route path="/settings" component={Settings} />
      <Route path="/device/:id" component={DeviceLayout}>
        <Route path="/" component={Overview} />
        <Route path="/apps" component={Apps} />
        <Route path="/install" component={Install} />
        <Route path="/themes" component={Themes} />
        <Route path="/remote" component={Remote} />
      </Route>
    </HashRouter>
  );
}

export default App;
