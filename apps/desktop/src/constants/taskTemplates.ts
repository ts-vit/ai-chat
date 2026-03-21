import {
    IconWorldSearch,
    IconSubtask,
    IconPencil,
} from "@tabler/icons-react";

export interface TemplateField {
    id: string;
    type: "text" | "textarea" | "radio" | "checkbox";
    label: string;
    placeholder?: string;
    options?: { value: string; label: string }[];
    defaultValue?: string | boolean;
    required?: boolean;
}

export interface TaskTemplate {
    id: string;
    titleKey: string;
    icon: React.FC<{ size?: number; stroke?: number; style?: React.CSSProperties }>;
    color: string;
    fields: TemplateField[];
}

export type TaskOptions = Record<string, string | boolean>;

export const TASK_TEMPLATES: TaskTemplate[] = [
    {
        id: "research",
        titleKey: "assistantDashboard.research",
        icon: IconWorldSearch,
        color: "blue",
        fields: [
            {
                id: "topic",
                type: "text",
                label: "taskTemplate.researchTopic",
                placeholder: "taskTemplate.researchTopicPlaceholder",
                required: true,
            },
            {
                id: "depth",
                type: "radio",
                label: "taskTemplate.depth",
                options: [
                    { value: "quick", label: "taskTemplate.depthQuick" },
                    { value: "detailed", label: "taskTemplate.depthDetailed" },
                    { value: "deep", label: "taskTemplate.depthDeep" },
                ],
                defaultValue: "detailed",
            },
            {
                id: "includeSources",
                type: "checkbox",
                label: "taskTemplate.includeSources",
                defaultValue: true,
            },
            {
                id: "saveToWorkspace",
                type: "checkbox",
                label: "taskTemplate.saveToWorkspace",
                defaultValue: false,
            },
        ],
    },
    {
        id: "plan",
        titleKey: "assistantDashboard.makePlan",
        icon: IconSubtask,
        color: "violet",
        fields: [
            {
                id: "goal",
                type: "textarea",
                label: "taskTemplate.planGoal",
                placeholder: "taskTemplate.planGoalPlaceholder",
                required: true,
            },
            {
                id: "complexity",
                type: "radio",
                label: "taskTemplate.complexity",
                options: [
                    { value: "simple", label: "taskTemplate.complexitySimple" },
                    { value: "medium", label: "taskTemplate.complexityMedium" },
                    { value: "complex", label: "taskTemplate.complexityComplex" },
                ],
                defaultValue: "medium",
            },
            {
                id: "executeImmediately",
                type: "checkbox",
                label: "taskTemplate.executeImmediately",
                defaultValue: false,
            },
        ],
    },
    {
        id: "write",
        titleKey: "assistantDashboard.writeDocument",
        icon: IconPencil,
        color: "orange",
        fields: [
            {
                id: "topic",
                type: "text",
                label: "taskTemplate.writeTopic",
                placeholder: "taskTemplate.writeTopicPlaceholder",
                required: true,
            },
            {
                id: "format",
                type: "radio",
                label: "taskTemplate.format",
                options: [
                    { value: "report", label: "taskTemplate.formatReport" },
                    { value: "article", label: "taskTemplate.formatArticle" },
                    { value: "code", label: "taskTemplate.formatCode" },
                    { value: "email", label: "taskTemplate.formatEmail" },
                ],
                defaultValue: "report",
            },
            {
                id: "useKb",
                type: "checkbox",
                label: "taskTemplate.useKb",
                defaultValue: false,
            },
        ],
    },
];

export function buildPromptFromTemplate(templateId: string, options: TaskOptions): string {
    switch (templateId) {
        case "research": {
            const depth =
                options.depth === "quick"
                    ? "brief overview"
                    : options.depth === "deep"
                      ? "comprehensive deep-dive research"
                      : "detailed analysis";
            let prompt = `Search the web and provide a ${depth}: ${options.topic}`;
            if (options.includeSources) prompt += "\n\nInclude sources and citations for key claims.";
            if (options.saveToWorkspace) prompt += "\n\nSave the final report to the workspace.";
            return prompt;
        }
        case "plan": {
            const complexity =
                options.complexity === "simple"
                    ? "3-5 tasks"
                    : options.complexity === "complex"
                      ? "10+ detailed tasks"
                      : "5-10 tasks";
            let prompt = `/plan ${options.goal}\n\nAim for ${complexity}.`;
            if (options.executeImmediately) prompt += "\nExecute the plan immediately after creation.";
            return prompt;
        }
        case "write": {
            const format =
                options.format === "report"
                    ? "a detailed report"
                    : options.format === "article"
                      ? "an article"
                      : options.format === "code"
                        ? "code/script"
                        : "an email/letter";
            let prompt = `Write ${format} about: ${options.topic}`;
            if (options.useKb) prompt += "\n\nUse the attached Knowledge Base for context and references.";
            prompt += "\n\nSave the result to the workspace.";
            return prompt;
        }
        default:
            return String(options.topic || "");
    }
}
