import React, { useEffect, useState } from 'react';
import { View, Text, StyleSheet, ScrollView, ActivityIndicator } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import { fetchNodeDetails } from '../services/api';

const NodeDetailScreen = ({ route }) => {
    const { address } = route.params;
    const [node, setNode] = useState(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        const loadNode = async () => {
            try {
                const data = await fetchNodeDetails(address);
                setNode(data);
            } catch (error) {
                console.error(error);
            } finally {
                setLoading(false);
            }
        };
        loadNode();
    }, [address]);

    if (loading) {
        return (
            <View style={styles.center}>
                <ActivityIndicator size="large" color="#6200ee" />
            </View>
        );
    }

    if (!node) {
        return (
            <View style={styles.center}>
                <Text>Node not found</Text>
            </View>
        );
    }

    return (
        <SafeAreaView style={styles.container} edges={['bottom', 'left', 'right']}>
            <ScrollView style={styles.scrollView}>
                <View style={styles.card}>
                    <Text style={styles.label}>Address</Text>
                    <Text style={styles.value}>{node.address}</Text>

                    <Text style={styles.label}>User Agent</Text>
                    <Text style={styles.value}>{node.soft || 'N/A'}</Text>

                    <Text style={styles.label}>Country</Text>
                    <Text style={styles.value}>{node.country || 'N/A'}</Text>

                    <Text style={styles.label}>Last Detected</Text>
                    <Text style={styles.value}>
                        {node.detected ? new Date(node.detected).toLocaleString() : 'N/A'}
                    </Text>
                </View>
            </ScrollView>
        </SafeAreaView>
    );
};

const styles = StyleSheet.create({
    container: {
        flex: 1,
        backgroundColor: '#f5f5f5',
    },
    scrollView: {
        padding: 16,
    },
    center: {
        flex: 1,
        justifyContent: 'center',
        alignItems: 'center',
    },
    card: {
        backgroundColor: 'white',
        borderRadius: 12,
        padding: 20,
        elevation: 2,
    },
    label: {
        fontSize: 14,
        color: '#666',
        marginTop: 12,
    },
    value: {
        fontSize: 18,
        fontWeight: 'bold',
        color: '#333',
        marginTop: 4,
    },
});

export default NodeDetailScreen;
