import React, { useState } from 'react';
import { View, Text, TextInput, TouchableOpacity, StyleSheet, ScrollView, ActivityIndicator } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import { fetchNodeDetails } from '../services/api';

const CheckNodeScreen = () => {
    const [address, setAddress] = useState('');
    const [node, setNode] = useState(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState('');

    const handleCheck = async () => {
        if (!address.trim()) return;
        setLoading(true);
        setError('');
        setNode(null);
        try {
            const data = await fetchNodeDetails(address.trim());
            if (data) {
                setNode(data);
            } else {
                setError('Node not found');
            }
        } catch (err) {
            setError('Error fetching node details');
            console.error(err);
        } finally {
            setLoading(false);
        }
    };

    return (
        <SafeAreaView style={styles.container} edges={['bottom', 'left', 'right']}>
            <View style={styles.inputContainer}>
                <TextInput
                    style={styles.input}
                    placeholder="Enter node address"
                    placeholderTextColor="#888"
                    value={address}
                    onChangeText={setAddress}
                    autoCapitalize="none"
                    autoCorrect={false}
                />
                <TouchableOpacity style={styles.button} onPress={handleCheck} disabled={loading}>
                    {loading ? (
                        <ActivityIndicator color="white" size="small" />
                    ) : (
                        <Text style={styles.buttonText}>Check</Text>
                    )}
                </TouchableOpacity>
            </View>

            <ScrollView style={styles.resultContainer}>
                {error ? <Text style={styles.errorText}>{error}</Text> : null}

                {node && (
                    <View style={styles.details}>
                        <Text style={styles.detailRow}><Text style={styles.label}>Address: </Text>{node.address}</Text>
                        <Text style={styles.detailRow}><Text style={styles.label}>User Agent: </Text>{node.soft || 'N/A'}</Text>
                        <Text style={styles.detailRow}><Text style={styles.label}>Country: </Text>{node.country || 'N/A'}</Text>
                        <Text style={styles.detailRow}><Text style={styles.label}>Last Detected: </Text>{node.detected ? new Date(node.detected).toLocaleString() : 'N/A'}</Text>

                    </View>
                )}
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
    inputContainer: {
        flexDirection: 'row',
        marginBottom: 20,
    },
    input: {
        flex: 1,
        backgroundColor: 'white',
        borderRadius: 8,
        padding: 12,
        marginRight: 10,
        borderWidth: 1,
        borderColor: '#ddd',
        fontSize: 16,
    },
    button: {
        backgroundColor: '#2C2C2C',
        paddingHorizontal: 20,
        justifyContent: 'center',
        borderRadius: 8,
    },
    buttonText: {
        color: 'white',
        fontWeight: 'bold',
        fontSize: 16,
    },
    resultContainer: {
        flex: 1,
    },
    errorText: {
        color: 'red',
        fontSize: 16,
        marginBottom: 10,
    },
    details: {
        backgroundColor: 'white',
        padding: 16,
        borderRadius: 8,
        elevation: 2,
    },
    detailRow: {
        fontSize: 16,
        marginBottom: 8,
        color: '#333',
    },
    label: {
        fontWeight: 'bold',
    },
});

export default CheckNodeScreen;
