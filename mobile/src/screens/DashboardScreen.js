import React, { useEffect, useState } from 'react';
import { View, Text, StyleSheet, ScrollView, RefreshControl, ActivityIndicator, Dimensions, TouchableOpacity, TextInput, Alert, Image } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import { fetchStats, fetchHistoricalStats, fetchIncomingStats } from '../services/api';
import { LineChart } from 'react-native-chart-kit';

const screenWidth = Dimensions.get('window').width;

const DashboardScreen = ({ navigation }) => {
    const [stats, setStats] = useState(null);
    const [history, setHistory] = useState([]);
    const [i2pNodes, setI2pNodes] = useState(0);
    const [loading, setLoading] = useState(true);
    const [refreshing, setRefreshing] = useState(false);
    const [nodeAddress, setNodeAddress] = useState('');

    const [timeRange, setTimeRange] = useState('24h');

    const handleCheckNode = () => {
        if (!nodeAddress.trim()) {
            Alert.alert('Error', 'Please enter a valid address');
            return;
        }
        navigation.navigate('NodeDetail', { address: nodeAddress.trim() });
    };

    const loadData = async (range = '24h') => {
        try {
            const [statsData, historyData, incomingData] = await Promise.all([
                fetchStats(),
                fetchHistoricalStats(range),
                fetchIncomingStats()
            ]);
            setStats(statsData);
            setHistory(historyData);


            const i2pStat = incomingData.find(s => s.protocol === 'i2p');
            setI2pNodes(i2pStat ? i2pStat.total_nodes : 0);

        } catch (error) {
            console.error(error);
        } finally {
            setLoading(false);
            setRefreshing(false);
        }
    };

    useEffect(() => {
        loadData(timeRange);
    }, [timeRange]);

    const onRefresh = () => {
        setRefreshing(true);
        loadData(timeRange);
    };

    if (loading) {
        return (
            <View style={styles.center}>
                <ActivityIndicator size="large" color="#2C2C2C" />
            </View>
        );
    }

    const chartData = {
        labels: history.map((h, i) => {

            const date = new Date(h.snapshot_time);
            if (timeRange === '24h') {
                return i % 4 === 0 ? `${date.getHours()}:00` : "";
            } else if (timeRange === '1w') {
                return i % 7 === 0 ? `${date.getDate()}/${date.getMonth() + 1}` : "";
            } else if (timeRange === '1m') {
                return i % 5 === 0 ? `${date.getDate()}/${date.getMonth() + 1}` : "";
            } else {
                return i % 4 === 0 ? `${date.getMonth() + 1}/${date.getFullYear().toString().substr(2)}` : "";
            }
        }),
        datasets: [
            {
                data: history.map(h => h.incoming_nodes || 0),
                color: (opacity = 1) => `rgba(44, 44, 44, ${opacity})`,
                strokeWidth: 2
            }
        ],
        legend: ["Listening Nodes"]
    };


    const latestSnapshot = history.length > 0 ? history[history.length - 1] : null;
    let clientTrendData = null;

    if (latestSnapshot && latestSnapshot.top_software) {


        const softwareList = latestSnapshot.top_software;
        const top5 = softwareList.slice(0, 5).map(s => s.soft);

        const datasets = top5.map((clientName, index) => {
            const data = history.map(snapshot => {
                if (!snapshot.top_software) return 0;

                if (!Array.isArray(snapshot.top_software)) return 0;

                const clientStats = snapshot.top_software.find(s => s.soft === clientName);
                return clientStats ? Number(clientStats.node_count) : 0;
            });

            const colors = [
                (opacity = 1) => `rgba(255, 99, 132, ${opacity})`,
                (opacity = 1) => `rgba(54, 162, 235, ${opacity})`,
                (opacity = 1) => `rgba(255, 206, 86, ${opacity})`,
                (opacity = 1) => `rgba(75, 192, 192, ${opacity})`,
                (opacity = 1) => `rgba(153, 102, 255, ${opacity})`,
            ];

            return {
                data,
                color: colors[index % colors.length],
                strokeWidth: 2,
                legend: clientName
            };
        });

        clientTrendData = {
            labels: chartData.labels,
            datasets: datasets,
            legend: []
        };
    }

    return (
        <SafeAreaView style={{ flex: 1, backgroundColor: '#f5f5f5' }} edges={['bottom', 'left', 'right']}>
            <ScrollView
                style={styles.container}
                refreshControl={<RefreshControl refreshing={refreshing} onRefresh={onRefresh} />}
            >
                <Text style={styles.header}>Bitcoin Network Overview</Text>

                <View style={styles.card}>
                    <Text style={styles.cardTitle}>Listening Nodes</Text>
                    <Text style={styles.bigNumber}>{stats?.incoming_nodes || 0}</Text>
                </View>

                <View style={styles.smallStatsContainer}>
                    <Text style={styles.smallStatsText}>Total Nodes: {stats?.total_nodes || 0} (listening and no listening nodes)</Text>
                </View>

                <View style={styles.row}>
                    <View style={[styles.card, styles.halfCard]}>
                        <Text style={styles.cardTitle}>IPv4</Text>
                        <Text style={styles.number}>{stats?.ipv4_nodes || 0}</Text>
                    </View>
                    <View style={[styles.card, styles.halfCard]}>
                        <Text style={styles.cardTitle}>IPv6</Text>
                        <Text style={styles.number}>{stats?.ipv6_nodes || 0}</Text>
                    </View>
                </View>

                <View style={styles.row}>
                    <View style={[styles.card, styles.halfCard]}>
                        <Text style={styles.cardTitle}>Tor</Text>
                        <Text style={styles.number}>{stats?.tor_nodes || 0}</Text>
                    </View>
                    <View style={[styles.card, styles.halfCard]}>
                        <Text style={styles.cardTitle}>I2P</Text>
                        <Text style={styles.number}>{i2pNodes}</Text>
                    </View>
                </View>


                <View style={styles.rangeContainer}>
                    {['24h', '1w', '1m', '1y'].map((range) => (
                        <TouchableOpacity
                            key={range}
                            style={[styles.rangeButton, timeRange === range && styles.rangeButtonActive]}
                            onPress={() => setTimeRange(range)}
                        >
                            <Text style={[styles.rangeText, timeRange === range && styles.rangeTextActive]}>
                                {range.toUpperCase()}
                            </Text>
                        </TouchableOpacity>
                    ))}
                </View>

                <Text style={styles.subHeader}>Activity (Listening Nodes)</Text>
                {history.length > 0 ? (
                    <LineChart
                        data={chartData}
                        width={screenWidth - 32}
                        height={220}
                        chartConfig={{
                            backgroundColor: "#ffffff",
                            backgroundGradientFrom: "#ffffff",
                            backgroundGradientTo: "#ffffff",
                            decimalPlaces: 0,
                            color: (opacity = 1) => `rgba(44, 44, 44, ${opacity})`,
                            labelColor: (opacity = 1) => `rgba(0, 0, 0, ${opacity})`,
                            style: {
                                borderRadius: 16
                            },
                            propsForDots: {
                                r: 4,
                                strokeWidth: 2,
                                stroke: "#2C2C2C"
                            }
                        }}
                        bezier={true}
                        withDots={false}
                        withShadow={true}
                        withInnerLines={true}
                        withOuterLines={true}
                        withVerticalLines={true}
                        withHorizontalLines={true}
                        withVerticalLabels={true}
                        withHorizontalLabels={true}
                        fromZero={false}
                        style={{
                            marginVertical: 8,
                            borderRadius: 16,
                            alignSelf: 'center'
                        }}
                    />
                ) : (
                    <Text style={styles.noData}>No historical data available</Text>
                )}

                <Text style={styles.subHeader}>Top 5 Clients Trend</Text>
                {clientTrendData ? (
                    <LineChart
                        data={clientTrendData}
                        width={screenWidth - 32}
                        height={300}
                        chartConfig={{
                            backgroundColor: "#ffffff",
                            backgroundGradientFrom: "#ffffff",
                            backgroundGradientTo: "#ffffff",
                            decimalPlaces: 0,
                            color: (opacity = 1) => `rgba(0, 0, 0, ${opacity})`,
                            labelColor: (opacity = 1) => `rgba(0, 0, 0, ${opacity})`,
                            propsForDots: {
                                r: "3",
                                strokeWidth: "1",
                            },
                            useShadowColorFromDataset: true
                        }}
                        bezier
                        style={{
                            marginVertical: 8,
                            borderRadius: 16,
                            alignSelf: 'center'
                        }}
                    />
                ) : (
                    <Text style={styles.noData}>No client data available</Text>
                )}


                {clientTrendData && (
                    <View style={styles.legendContainer}>
                        {clientTrendData.datasets.map((dataset, index) => (
                            <View key={index} style={styles.legendItem}>
                                <View style={[styles.legendColor, { backgroundColor: dataset.color(1) }]} />
                                <Text style={styles.legendText}>{dataset.legend}</Text>
                            </View>
                        ))}
                    </View>
                )}

                <View style={styles.checkNodeContainer}>
                    <TouchableOpacity
                        style={styles.checkButton}
                        onPress={() => navigation.navigate('CheckNode')}
                    >
                        <Text style={styles.checkButtonText}>Check a Node</Text>
                    </TouchableOpacity>
                </View>
            </ScrollView>
        </SafeAreaView>
    );
};

const styles = StyleSheet.create({
    container: {
        flex: 1,
        backgroundColor: '#f5f5f5',
        padding: 16,
    },
    center: {
        flex: 1,
        justifyContent: 'center',
        alignItems: 'center',
    },
    header: {
        fontSize: 28,
        fontWeight: 'bold',
        marginBottom: 16,
        color: '#333',
        textAlign: 'center',
    },
    imageContainer: {
        alignItems: 'center',
        marginVertical: 20,
    },
    logoImage: {
        width: 120,
        height: 120,
        borderRadius: 20,
    },
    subHeader: {
        fontSize: 20,
        fontWeight: 'bold',
        marginTop: 24,
        marginBottom: 12,
        color: '#333',
    },
    card: {
        backgroundColor: 'white',
        borderRadius: 12,
        padding: 16,
        marginBottom: 12,
        elevation: 2,
        shadowColor: '#000',
        shadowOffset: { width: 0, height: 2 },
        shadowOpacity: 0.1,
        shadowRadius: 4,
    },
    row: {
        flexDirection: 'row',
        justifyContent: 'space-between',
    },
    halfCard: {
        width: '48%',
    },
    cardTitle: {
        fontSize: 14,
        color: '#666',
        marginBottom: 4,
    },
    bigNumber: {
        fontSize: 36,
        fontWeight: 'bold',
        color: '#6200ee',
    },
    number: {
        fontSize: 24,
        fontWeight: 'bold',
        color: '#333',
    },
    noData: {
        textAlign: 'center',
        color: '#999',
        marginTop: 20,
    },
    button: {
        backgroundColor: '#2C2C2C',
        padding: 16,
        borderRadius: 12,
        alignItems: 'center',
        marginTop: 24,
        marginBottom: 12,
    },
    secondaryButton: {
        backgroundColor: 'white',
        borderWidth: 1,
        borderColor: '#2C2C2C',
        marginTop: 0,
        marginBottom: 32,
    },
    buttonText: {
        color: 'white',
        fontSize: 16,
        fontWeight: 'bold',
    },
    secondaryButtonText: {
        color: '#6200ee',
    },
    checkNodeContainer: {
        marginTop: 24,
        marginBottom: 12,
    },
    checkNodeTitle: {
        fontSize: 18,
        fontWeight: 'bold',
        marginBottom: 8,
        color: '#333',
    },
    inputContainer: {
        flexDirection: 'row',
        alignItems: 'center',
    },
    input: {
        flex: 1,
        backgroundColor: 'white',
        borderRadius: 8,
        padding: 12,
        marginRight: 8,
        borderWidth: 1,
        borderColor: '#ddd',
        fontSize: 16,
    },
    checkButton: {
        backgroundColor: '#2C2C2C',
        paddingVertical: 12,
        paddingHorizontal: 20,
        borderRadius: 8,
    },
    checkButtonText: {
        color: 'white',
        fontWeight: 'bold',
        fontSize: 16,
    },
    smallStatsContainer: {
        alignItems: 'center',
        marginBottom: 16,
    },
    smallStatsText: {
        fontSize: 14,
        color: '#666',
        fontStyle: 'italic',
    },
    legendContainer: {
        flexDirection: 'column',
        alignItems: 'flex-start',
        marginTop: 8,
        paddingHorizontal: 16,
    },
    legendItem: {
        flexDirection: 'row',
        alignItems: 'center',
        marginBottom: 8,
    },
    legendColor: {
        width: 12,
        height: 12,
        borderRadius: 6,
        marginRight: 8,
    },
    legendText: {
        fontSize: 12,
        color: '#333',
    },
    rangeContainer: {
        flexDirection: 'row',
        justifyContent: 'center',
        marginBottom: 16,
        marginTop: 8,
    },
    rangeButton: {
        paddingVertical: 6,
        paddingHorizontal: 12,
        borderRadius: 20,
        backgroundColor: '#e0e0e0',
        marginHorizontal: 4,
    },
    rangeButtonActive: {
        backgroundColor: '#2C2C2C',
    },
    rangeText: {
        fontSize: 14,
        color: '#333',
    },
    rangeTextActive: {
        color: 'white',
        fontWeight: 'bold',
    },
});

export default DashboardScreen;
