/** Defines application theme data independently of generated native bridge types. */
export type CustomPalette = {
  background: string;
  surface: string;
  text: string;
  accent: string;
};

export type CustomTheme = {
  id: string;
  name: string;
  light: CustomPalette;
  dark: CustomPalette;
};

/** Fully resolved tokens after the Rust registry applies inheritance and references. */
export type StyleTheme = {
  id: string;
  name: string;
  names: Record<string, string>;
  material: string;
  cssVars: {
    touch: Record<string, string>;
    light: Record<string, string>;
    dark: Record<string, string>;
  };
};
