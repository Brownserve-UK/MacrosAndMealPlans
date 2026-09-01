import AdminPanelSettingsIcon from '@mui/icons-material/AdminPanelSettingsOutlined';
import DarkModeIcon from '@mui/icons-material/DarkModeOutlined';
import CalendarMonthIcon from '@mui/icons-material/CalendarMonthOutlined';
import MenuBookIcon from '@mui/icons-material/MenuBookOutlined';
import InventoryIcon from '@mui/icons-material/Inventory2Outlined';
import KitchenIcon from '@mui/icons-material/KitchenOutlined';
import LightModeIcon from '@mui/icons-material/LightModeOutlined';
import LogoutIcon from '@mui/icons-material/LogoutOutlined';
import PeopleIcon from '@mui/icons-material/PeopleOutlined';
import PersonIcon from '@mui/icons-material/PersonOutlined';
import RestaurantIcon from '@mui/icons-material/RestaurantOutlined';
import RestaurantMenuIcon from '@mui/icons-material/RestaurantMenuOutlined';
import AppBar from '@mui/material/AppBar';
import Divider from '@mui/material/Divider';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Avatar from '@mui/material/Avatar';
import Box from '@mui/material/Box';
import Container from '@mui/material/Container';
import Drawer from '@mui/material/Drawer';
import IconButton from '@mui/material/IconButton';
import List from '@mui/material/List';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import Toolbar from '@mui/material/Toolbar';
import Tooltip from '@mui/material/Tooltip';
import Typography from '@mui/material/Typography';
import { useColorScheme } from '@mui/material/styles';
import { Link, useNavigate, Outlet, useRouterState } from '@tanstack/react-router';
import { useState } from 'react';
import { useAuth } from '../auth/AuthProvider';

export const DRAWER_WIDTH = 236;

const NAV = [
  { to: '/food-log', label: 'Food log', icon: <MenuBookIcon fontSize="small" />, needs: undefined },
  { to: '/planner', label: 'My planner', icon: <CalendarMonthIcon fontSize="small" />, needs: undefined },
  { to: '/household/planner', label: 'Household planner', icon: <CalendarMonthIcon fontSize="small" />, needs: 'household:write' },
  { to: '/ingredients', label: 'Ingredients', icon: <RestaurantIcon fontSize="small" />, needs: 'catalogue:read' },
  { to: '/products', label: 'Products', icon: <InventoryIcon fontSize="small" />, needs: 'catalogue:read' },
  { to: '/recipes', label: 'Recipes', icon: <RestaurantMenuIcon fontSize="small" />, needs: 'catalogue:read' },
  { to: '/stock', label: 'Stock', icon: <KitchenIcon fontSize="small" />, needs: 'stock:read' },
  { to: '/household', label: 'Household', icon: <PeopleIcon fontSize="small" />, needs: 'household:read' },
] as const;

const BOTTOM_NAV = [
  {
    to: '/administration',
    label: 'Administration',
    icon: <AdminPanelSettingsIcon fontSize="small" />,
    needs: 'account:admin',
  },
] as const;

function UserMenu() {
  const { principal, signOut } = useAuth();
  const navigate = useNavigate();
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  if (!principal) return null;

  return (
    <>
      <IconButton onClick={(e) => setAnchor(e.currentTarget)} size="small" aria-label="Account">
        <Avatar sx={{ width: 30, height: 30, fontSize: 13 }}>
          {principal.username.slice(0, 1).toUpperCase()}
        </Avatar>
      </IconButton>
      <Menu anchorEl={anchor} open={Boolean(anchor)} onClose={() => setAnchor(null)}>
        <Typography variant="caption" color="text.secondary" sx={{ px: 2, py: 0.5, display: 'block' }}>
          {principal.username}
        </Typography>
        <Divider />
        <MenuItem
          onClick={() => {
            setAnchor(null);
            void navigate({ to: '/profile' });
          }}
        >
          <ListItemIcon>
            <PersonIcon fontSize="small" />
          </ListItemIcon>
          <ListItemText>Profile</ListItemText>
        </MenuItem>
        <MenuItem
          onClick={() => {
            setAnchor(null);
            signOut();
          }}
        >
          <ListItemIcon>
            <LogoutIcon fontSize="small" />
          </ListItemIcon>
          <ListItemText>Sign out</ListItemText>
        </MenuItem>
      </Menu>
    </>
  );
}

function ThemeToggle() {
  const { mode, setMode } = useColorScheme();
  const next = mode === 'dark' ? 'light' : 'dark';
  return (
    <Tooltip title={`Switch to ${next} mode`}>
      <IconButton onClick={() => setMode(next)} size="small">
        {mode === 'dark' ? <LightModeIcon fontSize="small" /> : <DarkModeIcon fontSize="small" />}
      </IconButton>
    </Tooltip>
  );
}

export function AppShell() {
  const { principal } = useAuth();
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  const activeNavTo = [...NAV, ...BOTTOM_NAV]
    .filter((item) => pathname === item.to || pathname.startsWith(`${item.to}/`))
    .sort((a, b) => b.to.length - a.to.length)[0]?.to;

  return (
    <Box sx={{ display: 'flex', minHeight: '100vh' }}>
      <Drawer
        variant="permanent"
        sx={{
          width: DRAWER_WIDTH,
          flexShrink: 0,
          '& .MuiDrawer-paper': {
            width: DRAWER_WIDTH,
            boxSizing: 'border-box',
            display: 'flex',
            flexDirection: 'column',
          },
          display: { xs: 'none', md: 'block' },
        }}
      >
        <Toolbar sx={{ px: 3, minHeight: 76 }}>
          <Typography variant="h2" sx={{ fontSize: '1.05rem', lineHeight: 1.25 }}>
            Macros &amp; Meal Plans
          </Typography>
        </Toolbar>
        <List sx={{ px: 1.5 }}>
          {NAV.filter((item) => !item.needs || (principal?.permissions.includes(item.needs) ?? false)).map((item) => (
            <ListItemButton
              key={item.to}
              component={Link}
              to={item.to}
              selected={item.to === activeNavTo}
              sx={{ mb: 0.5, py: 1 }}
            >
              <ListItemIcon sx={{ minWidth: 36 }}>{item.icon}</ListItemIcon>
              <ListItemText
                primary={item.label}
                slotProps={{ primary: { sx: { fontSize: 14, fontWeight: 500 } } }}
              />
            </ListItemButton>
          ))}
        </List>

        <Box sx={{ flexGrow: 1 }} />

        <List sx={{ px: 1.5, pb: 1.5 }}>
          {BOTTOM_NAV.filter((item) => principal?.permissions.includes(item.needs) ?? false).map(
            (item) => (
              <ListItemButton
                key={item.to}
                component={Link}
                to={item.to}
                selected={item.to === activeNavTo}
                sx={{ py: 1 }}
              >
                <ListItemIcon sx={{ minWidth: 36 }}>{item.icon}</ListItemIcon>
                <ListItemText
                  primary={item.label}
                  slotProps={{ primary: { sx: { fontSize: 14, fontWeight: 500 } } }}
                />
              </ListItemButton>
            ),
          )}
        </List>
      </Drawer>

      <Box sx={{ flexGrow: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>
        <AppBar position="sticky">
          <Toolbar sx={{ gap: 1, minHeight: 76 }}>
            <Box sx={{ flexGrow: 1 }} />
            <ThemeToggle />
            {principal ? <UserMenu /> : null}
          </Toolbar>
        </AppBar>

        <Container maxWidth="md" sx={{ py: 5, flexGrow: 1 }}>
          <Outlet />
        </Container>
      </Box>
    </Box>
  );
}
