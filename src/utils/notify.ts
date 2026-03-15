import { notifications } from "@mantine/notifications";
import i18n from "../i18n";

export const notify = {
    success: (message: string) => {
        notifications.show({
            message,
            color: "green",
            autoClose: 3000,
        });
    },
    error: (message: string) => {
        notifications.show({
            title: i18n.t("notifications.error"),
            message,
            color: "red",
            autoClose: 5000,
        });
    },
    warning: (message: string, id?: string) => {
        notifications.show({
            ...(id && { id }),
            message,
            color: "yellow",
            autoClose: 4000,
        });
    },
    info: (message: string) => {
        notifications.show({
            message,
            color: "blue",
            autoClose: 3000,
        });
    },
};
