import { createRootRoute, createRoute, createRouter, redirect } from '@tanstack/react-router';
import { AppShell } from '../components/AppShell';
import { AccountsPage } from '../features/administration/AccountsPage';
import { AdministrationPage } from '../features/administration/AdministrationPage';
import { MealTimesPage } from '../features/administration/MealTimesPage';
import { HouseholdPage } from '../features/household/HouseholdPage';
import { MemberPage } from '../features/household/MemberPage';
import { MealPlanIndexRedirect } from '../features/meal-plan/MealPlanIndexRedirect';
import { MealPlanPage, defaultDayFor } from '../features/meal-plan/MealPlanPage';
import { IngredientPage } from '../features/ingredients/IngredientPage';
import { IngredientsPage } from '../features/ingredients/IngredientsPage';
import { ProductPage } from '../features/products/ProductPage';
import { ProductsPage } from '../features/products/ProductsPage';
import { RecipePage } from '../features/recipes/RecipePage';
import { EditRecipePage, NewRecipePage } from '../features/recipes/RecipeFormPage';
import { RecipesPage } from '../features/recipes/RecipesPage';
import { ProfilePage } from '../features/profile/ProfilePage';

const rootRoute = createRootRoute({ component: AppShell });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  beforeLoad: () => {
    throw redirect({ to: '/food-log' });
  },
});

const foodLogIndexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/food-log',
  component: () => <MealPlanIndexRedirect workspace="today" />,
});

const foodLogWeekRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/food-log/$weekStart',
  beforeLoad: ({ params }) => {
    throw redirect({
      to: '/food-log/$weekStart/$day',
      params: { weekStart: params.weekStart, day: defaultDayFor(params.weekStart) },
      replace: true,
    });
  },
});

const foodLogDayRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/food-log/$weekStart/$day',
  component: function ViewFoodLog() {
    const { weekStart, day } = foodLogDayRoute.useParams();
    return <MealPlanPage weekStart={weekStart} day={day} workspace="today" />;
  },
});

const plannerIndexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/planner',
  component: () => <MealPlanIndexRedirect workspace="planner" />,
});

const plannerWeekRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/planner/$weekStart',
  beforeLoad: ({ params }) => {
    throw redirect({
      to: '/planner/$weekStart/$day',
      params: { weekStart: params.weekStart, day: defaultDayFor(params.weekStart) },
      replace: true,
    });
  },
});

const plannerDayRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/planner/$weekStart/$day',
  component: function ViewPlanner() {
    const { weekStart, day } = plannerDayRoute.useParams();
    return <MealPlanPage weekStart={weekStart} day={day} workspace="planner" />;
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

const recipesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/recipes',
  component: RecipesPage,
});

const newRecipeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/recipes/new',
  component: NewRecipePage,
});

const editRecipeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/recipes/$id/edit',
  component: function EditRecipe() {
    const { id } = editRecipeRoute.useParams();
    return <EditRecipePage id={id} />;
  },
});

const recipeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/recipes/$id',
  component: function EditRecipe() {
    const { id } = recipeRoute.useParams();
    return <RecipePage id={id} />;
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

const mealTimesRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/administration/meal-times',
  component: MealTimesPage,
});

const profileRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/profile',
  component: ProfilePage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  foodLogIndexRoute,
  foodLogWeekRoute,
  foodLogDayRoute,
  plannerIndexRoute,
  plannerWeekRoute,
  plannerDayRoute,
  ingredientsRoute,
  ingredientRoute,
  productsRoute,
  productRoute,
  recipesRoute,
  newRecipeRoute,
  recipeRoute,
  editRecipeRoute,
  householdRoute,
  memberRoute,
  administrationRoute,
  accountsRoute,
  mealTimesRoute,
  profileRoute,
]);

export const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
