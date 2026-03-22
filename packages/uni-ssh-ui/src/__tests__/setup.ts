import "@testing-library/jest-dom";

// Mantine Combobox uses scrollIntoView which jsdom doesn't implement
Element.prototype.scrollIntoView = () => {};
