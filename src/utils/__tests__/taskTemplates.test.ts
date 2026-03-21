import { describe, test, expect } from "vitest";
import { buildPromptFromTemplate } from "../../constants/taskTemplates";

describe("buildPromptFromTemplate", () => {
    describe("research", () => {
        test("generates detailed research prompt with sources", () => {
            const prompt = buildPromptFromTemplate("research", {
                topic: "AI agents",
                depth: "detailed",
                includeSources: true,
                saveToWorkspace: false,
            });
            expect(prompt).toContain("AI agents");
            expect(prompt).toContain("detailed analysis");
            expect(prompt).toContain("sources and citations");
            expect(prompt).not.toContain("workspace");
        });

        test("generates quick overview", () => {
            const prompt = buildPromptFromTemplate("research", {
                topic: "quantum computing",
                depth: "quick",
                includeSources: false,
                saveToWorkspace: false,
            });
            expect(prompt).toContain("brief overview");
            expect(prompt).toContain("quantum computing");
            expect(prompt).not.toContain("sources");
        });

        test("generates deep research with workspace save", () => {
            const prompt = buildPromptFromTemplate("research", {
                topic: "climate change",
                depth: "deep",
                includeSources: true,
                saveToWorkspace: true,
            });
            expect(prompt).toContain("comprehensive deep-dive research");
            expect(prompt).toContain("Save the final report to the workspace");
            expect(prompt).toContain("sources and citations");
        });
    });

    describe("plan", () => {
        test("generates medium complexity plan", () => {
            const prompt = buildPromptFromTemplate("plan", {
                goal: "Redesign API",
                complexity: "medium",
                executeImmediately: false,
            });
            expect(prompt).toContain("/plan");
            expect(prompt).toContain("Redesign API");
            expect(prompt).toContain("5-10 tasks");
            expect(prompt).not.toContain("Execute");
        });

        test("generates simple plan with immediate execution", () => {
            const prompt = buildPromptFromTemplate("plan", {
                goal: "Fix login bug",
                complexity: "simple",
                executeImmediately: true,
            });
            expect(prompt).toContain("3-5 tasks");
            expect(prompt).toContain("Execute the plan immediately");
        });

        test("generates complex plan", () => {
            const prompt = buildPromptFromTemplate("plan", {
                goal: "Full rewrite",
                complexity: "complex",
                executeImmediately: false,
            });
            expect(prompt).toContain("10+ detailed tasks");
        });
    });

    describe("write", () => {
        test("generates report prompt", () => {
            const prompt = buildPromptFromTemplate("write", {
                topic: "quarterly results",
                format: "report",
                useKb: false,
            });
            expect(prompt).toContain("a detailed report");
            expect(prompt).toContain("quarterly results");
            expect(prompt).toContain("Save the result to the workspace");
            expect(prompt).not.toContain("Knowledge Base");
        });

        test("generates article with KB context", () => {
            const prompt = buildPromptFromTemplate("write", {
                topic: "machine learning trends",
                format: "article",
                useKb: true,
            });
            expect(prompt).toContain("an article");
            expect(prompt).toContain("Knowledge Base");
        });

        test("generates code prompt", () => {
            const prompt = buildPromptFromTemplate("write", {
                topic: "REST API server",
                format: "code",
                useKb: false,
            });
            expect(prompt).toContain("code/script");
        });

        test("generates email prompt", () => {
            const prompt = buildPromptFromTemplate("write", {
                topic: "project update",
                format: "email",
                useKb: false,
            });
            expect(prompt).toContain("an email/letter");
        });
    });

    test("unknown template returns topic", () => {
        const prompt = buildPromptFromTemplate("unknown", { topic: "test" });
        expect(prompt).toBe("test");
    });

    test("unknown template with no topic returns empty string", () => {
        const prompt = buildPromptFromTemplate("unknown", {});
        expect(prompt).toBe("");
    });
});
