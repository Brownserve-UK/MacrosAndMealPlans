import { createRootRoute, createRoute, createRouter, redirect } from '@tanstack/react-router';
import { AppShell } from '../components/AppShell';
import { RequirePermission } from '../components/RequirePermission';
import { RouteError, RouteNotFound } from '../components/RouteStates';
import { AccountsPage } from '../features/administration/AccountsPage';
import { AdministrationPage } from '../features/administration/AdministrationPage';
import { MealTimesPage } from '../features/administration/MealTimesPage';
import { HouseholdPage } from '../features/household/HouseholdPage';
import { MemberPage } from '../features/household/MemberPage';
import { MealPlanIndexRedirect } from '../features/meal-plan/MealPlanIndexRedirect';
import { MealPlanPage } from '../features/meal-plan/MealPlanPage';
import { MyPlannerPage } from '../features/meal-plan/MyPlannerPage';
import { NeedsReviewPage } from '../features/meal-plan/NeedsReviewPage';
import { HouseholdPlannerPage } from '../features/meal-plan/HouseholdPlannerPage';
import { defaultDayFor } from '../features/meal-plan/date';
import { IngredientPage } from '../features/ingredients/IngredientPage';
import { IngredientsPage } from '../features/ingredients/IngredientsPage';
import { ProductPage } from '../features/products/ProductPage';
import { ProductsPage } from '../features/products/ProductsPage';
import { RecipePage } from '../features/recipes/RecipePage';
import { EditRecipePage, NewRecipePage } from '../features/recipes/RecipeFormPage';
import { RecipesPage } from '../features/recipes/RecipesPage';
import { ProfilePage } from '../features/profile/ProfilePage';
import { StockPage } from '../features/stock/StockPage';
import { StockItemPage } from '../features/stock/StockItemPage';
import { ProductStockPage } from '../features/stock/ProductStockPage';
import { IngredientStockPage } from '../features/stock/IngredientStockPage';

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
  component: () => <MealPlanIndexRedirect to="/food-log" />,
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
    return <MealPlanPage weekStart={weekStart} day={day} />;
  },
});

const plannerIndexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/planner',
  component: () => <MealPlanIndexRedirect to="/planner" />,
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
    return <MyPlannerPage weekStart={weekStart} day={day} />;
  },
});

const householdPlannerIndexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/household/planner',
  component: () => (
    <RequirePermission permission="household:write">
      <MealPlanIndexRedirect to="/household/planner" />
    </RequirePermission>
  ),
});

const householdPlannerWeekRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/household/planner/$weekStart',
  beforeLoad: ({ params }) => {
    throw redirect({
      to: '/household/planner/$weekStart/$day',
      params: { weekStart: params.weekStart, day: defaultDayFor(params.weekStart) },
      replace: true,
    });
  },
});

const householdPlannerDayRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/household/planner/$weekStart/$day',
  component: function ViewHouseholdPlanner() {
    const { weekStart, day } = householdPlannerDayRoute.useParams();
    return (
      <RequirePermission permission="household:write">
        <HouseholdPlannerPage weekStart={weekStart} day={day} />
      </RequirePermission>
    );
  },
});

const needsReviewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/needs-review',
  component: NeedsReviewPage,
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

const stockRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/stock',
  component: StockPage,
});

const stockItemRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/stock/$id',
  component: function ViewStockItem() {
    const { id } = stockItemRoute.useParams();
    return <StockItemPage id={id} />;
  },
});

const productStockRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/stock/products/$productId',
  component: function ViewProductStock() {
    const { productId } = productStockRoute.useParams();
    return <ProductStockPage productId={productId} />;
  },
});

const ingredientStockRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/stock/ingredients/$ingredientId',
  component: function ViewIngredientStock() {
    const { ingredientId } = ingredientStockRoute.useParams();
    return <IngredientStockPage ingredientId={ingredientId} />;
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
  householdPlannerIndexRoute,
  householdPlannerWeekRoute,
  householdPlannerDayRoute,
  needsReviewRoute,
  ingredientsRoute,
  ingredientRoute,
  productsRoute,
  productRoute,
  recipesRoute,
  newRecipeRoute,
  recipeRoute,
  editRecipeRoute,
  stockRoute,
  productStockRoute,
  ingredientStockRoute,
  stockItemRoute,
  householdRoute,
  memberRoute,
  administrationRoute,
  accountsRoute,
  mealTimesRoute,
  profileRoute,
]);

export const router = createRouter({
  routeTree,
  defaultNotFoundComponent: RouteNotFound,
  defaultErrorComponent: RouteError,
});

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
