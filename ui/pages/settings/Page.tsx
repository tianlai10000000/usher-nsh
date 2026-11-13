/** Settings route entry; feature sections remain composed by the existing page. */
import { SettingsPage } from "../../features/settings/SettingsPage";
import type { PageProps } from "../contracts";
export function SettingsEntry(props: PageProps) { return <SettingsPage appearance={props.appearance} />; }
