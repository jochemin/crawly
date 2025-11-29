import React from 'react';
import { View, Text, Image } from 'react-native';
import { NavigationContainer } from '@react-navigation/native';
import { createStackNavigator } from '@react-navigation/stack';
import DashboardScreen from './src/screens/DashboardScreen';
import { StatusBar } from 'expo-status-bar';

import NodeListScreen from './src/screens/NodeListScreen';
import NodeDetailScreen from './src/screens/NodeDetailScreen';
import CheckNodeScreen from './src/screens/CheckNodeScreen';

const Stack = createStackNavigator();

export default function App() {
  return (
    <NavigationContainer>
      <StatusBar style="auto" />
      <Stack.Navigator
        initialRouteName="Dashboard"
        screenOptions={{
          headerStyle: {
            backgroundColor: '#2C2C2C',
          },
          headerTintColor: '#fff',
          headerTitleStyle: {
            fontWeight: 'bold',
          },
        }}
      >
        <Stack.Screen
          name="Dashboard"
          component={DashboardScreen}
          options={{
            headerTitle: () => (
              <View style={{ flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', width: '100%' }}>
                <Image
                  source={require('./assets/crawly_transparent.png')}
                  style={{ width: 30, height: 30, borderRadius: 5 }}
                />
                <Text style={{ color: '#fff', fontSize: 20, fontWeight: 'bold' }}>Crawly</Text>
                <Image
                  source={require('./assets/crawly_transparent.png')}
                  style={{ width: 30, height: 30, borderRadius: 5 }}
                />
              </View>
            ),
            headerTitleAlign: 'center',
            headerTitleContainerStyle: { width: '100%' },
            title: 'Crawly'
          }}
        />
        <Stack.Screen name="NodeList" component={NodeListScreen} options={{ title: 'Nodes' }} />
        <Stack.Screen name="NodeDetail" component={NodeDetailScreen} options={{ title: 'Node Details' }} />
        <Stack.Screen name="CheckNode" component={CheckNodeScreen} options={{ title: 'Check Node' }} />
      </Stack.Navigator>
    </NavigationContainer>
  );
}
