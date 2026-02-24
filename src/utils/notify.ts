import { notifications } from "@mantine/notifications";

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
            title: "Ошибка",
            message,
            color: "red",
            autoClose: 5000,
        });
    },
    warning: (message: string) => {
        notifications.show({
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
