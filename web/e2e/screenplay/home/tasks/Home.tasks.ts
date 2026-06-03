import { Interaction, Task, the } from "@serenity-js/core";
import { Navigate } from "@serenity-js/web";

import { nativePageFor } from "../../support/actor-page";
import { HomeScreen } from "../screens/Home.screen";

export const VisitHomePage = () =>
  Task.where(
    the`#actor visits the Elizabeth home page`,
    Navigate.to("/"),
  );

export const OpenCreateRoomForm = () =>
  Interaction.where(the`#actor opens the create room form`, async (actor) => {
    const page = await nativePageFor(actor);
    await HomeScreen.createRoomCard(page).click();
  });

export const OpenJoinRoomForm = () =>
  Interaction.where(the`#actor opens the join room form`, async (actor) => {
    const page = await nativePageFor(actor);
    await HomeScreen.joinRoomCard(page).click();
  });

export const CreateRoomFromHome = (
  roomName: string,
  options: {
    password?: string;
    confirmPassword?: string;
  } = {},
) =>
  Task.where(
    the`#actor creates the room called ${roomName}`,
    VisitHomePage(),
    OpenCreateRoomForm(),
    Interaction.where(the`#actor enters the room name`, async (actor) => {
      const page = await nativePageFor(actor);
      await HomeScreen.roomNameInput(page).fill(roomName);
    }),
    Interaction.where(the`#actor enters optional room credentials`, async (actor) => {
      const page = await nativePageFor(actor);
      if (options.password) {
        await HomeScreen.createPasswordInput(page).fill(options.password);
      }
      if (options.confirmPassword) {
        await HomeScreen.confirmPasswordInput(page).fill(options.confirmPassword);
      }
    }),
    Interaction.where(the`#actor submits the create room form`, async (actor) => {
      const page = await nativePageFor(actor);
      await HomeScreen.createRoomButton(page).click();
    }),
  );

export const StartJoiningRoom = (roomName: string) =>
  Task.where(
    the`#actor starts joining the room called ${roomName}`,
    VisitHomePage(),
    OpenJoinRoomForm(),
    Interaction.where(the`#actor enters the target room name`, async (actor) => {
      const page = await nativePageFor(actor);
      await HomeScreen.joinRoomNameInput(page).fill(roomName);
    }),
    Interaction.where(the`#actor submits the join room form`, async (actor) => {
      const page = await nativePageFor(actor);
      await HomeScreen.joinRoomButton(page).click();
    }),
  );

export const NavigateBackFromJoinForm = () =>
  Task.where(
    the`#actor returns from the join room form`,
    VisitHomePage(),
    OpenJoinRoomForm(),
    Interaction.where(the`#actor clicks the back button`, async (actor) => {
      const page = await nativePageFor(actor);
      await HomeScreen.backButton(page).click();
    }),
  );
