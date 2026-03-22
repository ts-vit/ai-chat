import "@testing-library/jest-dom";

// Mantine Combobox uses scrollIntoView which jsdom doesn't implement
Element.prototype.scrollIntoView = () => {};

// Mantine useColorScheme uses matchMedia which jsdom doesn't implement
Object.defineProperty(window, "matchMedia", {
    writable: true,
    value: (query: string) => ({
        matches: false,
        media: query,
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: () => {},
        removeEventListener: () => {},
        dispatchEvent: () => false,
    }),
});
