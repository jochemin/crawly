const BASE_URL = 'http://localhost:3000';

export const fetchStats = async () => {
  try {
    const response = await fetch(`${BASE_URL}/api/stats`);
    if (!response.ok) throw new Error('Network response was not ok');
    return await response.json();
  } catch (error) {
    console.error('Error fetching stats:', error);
    throw error;
  }
};

export const fetchRecentNodes = async (limit = 20, page = 1) => {
  try {
    const response = await fetch(`${BASE_URL}/api/nodes?limit=${limit}&page=${page}`);
    if (!response.ok) throw new Error('Network response was not ok');
    return await response.json();
  } catch (error) {
    console.error('Error fetching recent nodes:', error);
    throw error;
  }
};

export const fetchNodeDetails = async (address) => {
  try {
    const response = await fetch(`${BASE_URL}/api/node/${address}`);
    if (!response.ok) throw new Error('Network response was not ok');
    return await response.json();
  } catch (error) {
    console.error('Error fetching node details:', error);
    throw error;
  }
};

export const fetchSoftwareStats = async () => {
  try {
    const response = await fetch(`${BASE_URL}/api/software_stats`);
    if (!response.ok) throw new Error('Network response was not ok');
    return await response.json();
  } catch (error) {
    console.error('Error fetching software stats:', error);
    throw error;
  }
};

export const fetchIncomingStats = async () => {
  try {
    const response = await fetch(`${BASE_URL}/api/incoming_stats`);
    if (!response.ok) throw new Error('Network response was not ok');
    return await response.json();
  } catch (error) {
    console.error('Error fetching incoming stats:', error);
    throw error;
  }
};

export const fetchHistoricalStats = async (range = '24h') => {
  try {
    const response = await fetch(`${BASE_URL}/api/stats/history?range=${range}`);
    if (!response.ok) throw new Error('Network response was not ok');
    return await response.json();
  } catch (error) {
    console.error('Error fetching historical stats:', error);
    throw error;
  }
};

export const searchNodes = async (query) => {
  try {
    const response = await fetch(`${BASE_URL}/api/nodes/search?q=${encodeURIComponent(query)}`);
    if (!response.ok) throw new Error('Network response was not ok');
    return await response.json();
  } catch (error) {
    console.error('Error searching nodes:', error);
    throw error;
  }
};
