/// <reference types="@raycast/api">

/* 🚧 🚧 🚧
 * This file is auto-generated from the extension's manifest.
 * Do not modify manually. Instead, update the `package.json` file.
 * 🚧 🚧 🚧 */

/* eslint-disable @typescript-eslint/ban-types */

type ExtensionPreferences = {
  /** API Token (For Font Awesome Pro Accounts) - Optionally use Font Awesome Pro icons and custom kits by providing an API Token. Not necessary to access free icons. */
  "API_TOKEN"?: string,
  /** Icon family and style preference - Choose which icon family and style you prefer. */
  "STYLE_PREFERENCE": "fas" | "far" | "fal" | "fat" | "fab" | "fad" | "fass" | "fasr" | "fasl" | "fast",
  /** Icon primary action - Choose which action to perform when selecting an icon. */
  "PRIMARY_ACTION": "copyIconName" | "copyIconClasses" | "copyAsSvg" | "copyIconGlyph" | "copyIconUnicode",
  /** undefined - When enabled, the extension will open with your previously selected icon style or custom kit. */
  "REMEMBER_LAST_KIT": boolean,
  /** Filter Custom Kits - Limit which custom kits appear in the dropdown. Enter kit names or IDs separated by commas (e.g., "My Icons, abc123"). Leave empty to show all kits. */
  "KIT_FILTER"?: string
}

/** Preferences accessible in all the extension's commands */
declare type Preferences = ExtensionPreferences

declare namespace Preferences {
  /** Preferences accessible in the `index` command */
  export type Index = ExtensionPreferences & {}
}

declare namespace Arguments {
  /** Arguments passed to the `index` command */
  export type Index = {}
}

