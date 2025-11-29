import React, { useEffect, useState, useCallback } from 'react';
import { View, Text, StyleSheet, FlatList, ActivityIndicator, TouchableOpacity, TextInput } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';
import { fetchRecentNodes, searchNodes } from '../services/api';
import { useNavigation } from '@react-navigation/native';

const NodeListScreen = () => {
    const navigation = useNavigation();
    const [nodes, setNodes] = useState([]);
    const [loading, setLoading] = useState(false);
    const [page, setPage] = useState(1);
    const [hasMore, setHasMore] = useState(true);
    const [searchQuery, setSearchQuery] = useState('');
    const [isSearching, setIsSearching] = useState(false);

    const loadNodes = async (pageNum = 1, shouldRefresh = false) => {
        if (loading) return;
        setLoading(true);
        try {
            const newNodes = await fetchRecentNodes(20, pageNum);
            if (shouldRefresh) {
                setNodes(newNodes);
            } else {
                setNodes(prev => [...prev, ...newNodes]);
            }
            setHasMore(newNodes.length === 20);
            setPage(pageNum);
        } catch (error) {
            console.error(error);
        } finally {
            setLoading(false);
        }
    };

    const handleSearch = async () => {
        if (!searchQuery.trim()) {
            setIsSearching(false);
            loadNodes(1, true);
            return;
        }
        setLoading(true);
        setIsSearching(true);
        try {
            const results = await searchNodes(searchQuery);
            setNodes(results);
            setHasMore(false);
        } catch (error) {
            console.error(error);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        loadNodes();
    }, []);

    const renderItem = ({ item }) => (
        <TouchableOpacity
            style={styles.item}
            onPress={() => navigation.navigate('NodeDetail', { address: item.address })}
        >
            <View>
                <Text style={styles.address}>{item.address}</Text>
                <Text style={styles.subText}>{item.soft || 'Unknown Agent'}</Text>
            </View>
            <View style={styles.right}>
                <Text style={styles.country}>{item.country || '??'}</Text>
            </View>
        </TouchableOpacity>
    );

    const renderFooter = () => {
        if (!loading) return null;
        return <ActivityIndicator style={{ margin: 20 }} size="small" color="#6200ee" />;
    };

    return (
        <SafeAreaView style={styles.container} edges={['bottom', 'left', 'right']}>
            <View style={styles.searchContainer}>
                <TextInput
                    style={styles.searchInput}
                    placeholder="Search IP or Software..."
                    value={searchQuery}
                    onChangeText={setSearchQuery}
                    onSubmitEditing={handleSearch}
                    returnKeyType="search"
                />
            </View>
            <FlatList
                data={nodes}
                renderItem={renderItem}
                keyExtractor={(item) => item.address}
                onEndReached={() => {
                    if (hasMore && !loading && !isSearching) {
                        loadNodes(page + 1);
                    }
                }}
                onEndReachedThreshold={0.5}
                ListFooterComponent={renderFooter}
                refreshing={loading && page === 1}
                onRefresh={() => {
                    if (isSearching) {
                        handleSearch();
                    } else {
                        loadNodes(1, true);
                    }
                }}
            />
        </SafeAreaView>
    );
};

const styles = StyleSheet.create({
    container: {
        flex: 1,
        backgroundColor: '#f5f5f5',
    },
    searchContainer: {
        padding: 16,
        backgroundColor: 'white',
        borderBottomWidth: 1,
        borderBottomColor: '#eee',
    },
    searchInput: {
        backgroundColor: '#f0f0f0',
        padding: 10,
        borderRadius: 8,
        fontSize: 16,
    },
    item: {
        backgroundColor: 'white',
        padding: 16,
        borderBottomWidth: 1,
        borderBottomColor: '#eee',
        flexDirection: 'row',
        justifyContent: 'space-between',
        alignItems: 'center',
    },
    address: {
        fontSize: 16,
        fontWeight: 'bold',
        color: '#333',
    },
    subText: {
        fontSize: 14,
        color: '#666',
        marginTop: 4,
    },
    right: {
        alignItems: 'flex-end',
    },
    country: {
        fontSize: 16,
        fontWeight: 'bold',
        color: '#6200ee',
    },
});

export default NodeListScreen;
