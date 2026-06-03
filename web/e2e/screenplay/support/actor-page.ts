import type { Page as PlaywrightPage } from "@playwright/test";
import type { AnswersQuestions, UsesAbilities } from "@serenity-js/core";
import {
  BrowseTheWebWithPlaywright,
  PlaywrightPage as SerenityPlaywrightPage,
} from "@serenity-js/playwright";

export async function nativePageFor(
  actor: AnswersQuestions & UsesAbilities,
): Promise<PlaywrightPage> {
  const page = await BrowseTheWebWithPlaywright.as(actor)
    .currentPage() as SerenityPlaywrightPage;
  return page.nativePage();
}
