import { createRootRoute, createRoute, createRouter, redirect } from '@tanstack/react-router';
import { AppShell } from '../components/AppShell';
import { AccountsPage } from '../features/administration/AccountsPage';
import { AdministrationPage } from '../features/administration/AdministrationPage';
import { HouseholdPage } from '../features/household/HouseholdPage';
import { MemberPage } from '../features/household/MemberPage';
import { MealPlanIndexRedirect } from '../features/meal-plan/MealPlanIndexRedirect';
import { MealPlanPage, defaultDayFor } from '../features/meal-plan/MealPlanPage';
import { IngredientPage } from '../features/ingredients/IngredientPage';
import { IngredientsPage } from '../features/ingredients/IngredientsPage';
import { ProductPage } from '../features/products/ProductPage';
import { ProductsPage } from '../features/products/ProductsPage';
import { ProfilePage } from '../features/profile/ProfilePage';

const rootRoute = createRootRoute({ component: AppShell });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  beforeLoad: () => {
    throw redirect({ to: '/meal-plan' });
  },
});

const mealPlanIndexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/meal-plan',
  component: MealPlanIndexRedirect,
});

const mealPlanWeekRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/meal-plan/$weekStart',
  beforeLoad: ({ params }) => {
    throw redirect({
      to: '/meal-plan/$weekStart/$day',
      params: { weekStart: params.weekStart, day: defaultDayFor(params.weekStart) },
      replace: true,
    });
  },
});

const mealPlanDayRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/meal-plan/$weekStart/$day',
  component: function ViewMealPlanDay() {
    const { weekStart, day } = mealPlanDayRoute.useParams();
    return <MealPlanPage weekStart={weekStart} day={day} />;
  },
});

const ingredientsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/ingredients',
  component: IngredientsPage,
});

const ingredientRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/ingredients/$id',
  component: function EditIngredient() {
    const { id } = ingredientRoute.useParams();
    return <IngredientPage id={id} />;
  },
});

const productsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/products',
  component: ProductsPage,
});

const productRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/products/$id',
  component: function EditProduct() {
    const { id } = productRoute.useParams();
    return <ProductPage id={id} />;
  },
});

const householdRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/household',
  component: HouseholdPage,
});

const memberRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/household/$id',
  component: function EditMember() {
    const { id } = memberRoute.useParams();
    return <MemberPage id={id} />;
  },
});

const administrationRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/administration',
  component: AdministrationPage,
});

const accountsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/administration/accounts',
  component: AccountsPage,
});

const profileRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/profile',
  component: ProfilePage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  mealPlanIndexRoute,
  mealPlanWeekRoute,
  mealPlanDayRoute,
  ingredientsRoute,
  ingredientRoute,
  productsRoute,
  productRoute,
  householdRoute,
  memberRoute,
  administrationRoute,
  accountsRoute,
  profileRoute,
]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
