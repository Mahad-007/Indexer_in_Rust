-- Seed data for BeanBee frontend development
-- Based on mock-data.ts from frontend

-- Seed Tokens
INSERT INTO tokens (address, name, symbol, decimals, price_usd, price_change_1h, price_change_24h, liquidity_usd, market_cap_usd, volume_1h_usd, volume_24h_usd, holder_count, bee_score, safety_score, traction_score, lp_locked, dev_holdings_percent, sniper_ratio, created_at)
VALUES
    ('0x1234567890abcdef1234567890abcdef12345678', 'PepeCoin', 'PEPE', 18, 0.0000234, 15.4, 127.8, 125000, 2400000, 45000, 890000, 2453, 78, 48, 30, true, 5.2, 8.5, NOW() - INTERVAL '12 hours'),
    ('0xabcdef1234567890abcdef1234567890abcdef12', 'MoonDoge', 'MDOGE', 18, 0.000089, -5.2, 45.3, 89000, 1200000, 23000, 456000, 1823, 65, 38, 27, true, 8.1, 12.3, NOW() - INTERVAL '1 day'),
    ('0x9876543210fedcba9876543210fedcba98765432', 'SafeGains', 'SGAINS', 18, 0.00156, 8.7, 234.5, 234000, 5600000, 78000, 1230000, 4521, 85, 52, 33, true, 3.4, 5.2, NOW() - INTERVAL '2 days'),
    ('0xfedcba9876543210fedcba9876543210fedcba98', 'RocketFi', 'RKTFI', 18, 0.000012, -12.3, -25.6, 45000, 380000, 8900, 125000, 892, 42, 25, 17, false, 15.8, 22.1, NOW() - INTERVAL '6 hours'),
    ('0x5555666677778888999900001111222233334444', 'DegenApe', 'DAPE', 18, 0.00789, 45.2, 567.8, 567000, 12000000, 234000, 3400000, 8934, 92, 55, 37, true, 2.1, 3.8, NOW() - INTERVAL '3 days'),
    ('0xaaaa1111bbbb2222cccc3333dddd4444eeee5555', 'ElonMars', 'EMARS', 18, 0.0000045, 2.3, 18.9, 67000, 890000, 15000, 234000, 1234, 58, 35, 23, true, 9.5, 14.2, NOW() - INTERVAL '18 hours'),
    ('0x1111aaaa2222bbbb3333cccc4444dddd5555eeee', 'FlokiGold', 'FGOLD', 18, 0.000234, -8.9, 78.4, 178000, 3400000, 56000, 890000, 3456, 71, 42, 29, true, 6.7, 9.8, NOW() - INTERVAL '36 hours'),
    ('0x6666777788889999aaaa0000bbbbccccddddeeee', 'BabyShark', 'BSHARK', 18, 0.0000089, 23.4, 156.7, 98000, 1500000, 34000, 567000, 2134, 69, 40, 29, true, 7.3, 11.5, NOW() - INTERVAL '8 hours'),
    ('0xcccc1111dddd2222eeee3333ffff44445555aaaa', 'WenLambo', 'WLAMBO', 18, 0.00567, 67.8, 445.3, 345000, 7800000, 145000, 2100000, 5678, 88, 53, 35, true, 4.2, 6.1, NOW() - INTERVAL '60 hours'),
    ('0xddddeeeeffffaaaa11112222333344445555666', 'RugPull', 'RUG', 18, 0.0000001, -45.6, -89.2, 5000, 45000, 890, 12000, 234, 15, 8, 7, false, 45.2, 38.5, NOW() - INTERVAL '2 hours')
ON CONFLICT (address) DO NOTHING;

-- Seed Wallets
INSERT INTO wallets (address, label, token_count, estimated_value_usd, last_activity)
VALUES
    ('0xwhale123456789abcdef123456789abcdef12345', 'Smart Money Whale #1', 23, 2450000, NOW() - INTERVAL '2 hours'),
    ('0xtrader987654321fedcba987654321fedcba9876', 'Top BSC Trader', 45, 890000, NOW() - INTERVAL '1 hour'),
    ('0xdev111222333444555666777888999aaabbbccc', 'Known Dev Wallet', 12, 567000, NOW() - INTERVAL '12 hours'),
    ('0xsniper444555666777888999aaabbbcccdddeeef', 'Sniper Bot', 67, 345000, NOW() - INTERVAL '30 minutes')
ON CONFLICT (address) DO NOTHING;

-- Seed Wallet Activity
INSERT INTO wallet_activity (wallet_address, tx_hash, block_number, timestamp, action, token_address, token_symbol, amount_tokens, amount_usd)
VALUES
    ('0xwhale123456789abcdef123456789abcdef12345', '0xtx001', 1000001, NOW() - INTERVAL '15 minutes', 'buy', '0x1234567890abcdef1234567890abcdef12345678', 'PEPE', 50000000000, 12500),
    ('0xwhale123456789abcdef123456789abcdef12345', '0xtx002', 1000002, NOW() - INTERVAL '1 hour', 'sell', '0xabcdef1234567890abcdef1234567890abcdef12', 'MDOGE', 25000000000, 8900),
    ('0xtrader987654321fedcba987654321fedcba9876', '0xtx003', 1000003, NOW() - INTERVAL '30 minutes', 'buy', '0x5555666677778888999900001111222233334444', 'DAPE', 10000000, 45000),
    ('0xsniper444555666777888999aaabbbcccdddeeef', '0xtx004', 1000004, NOW() - INTERVAL '2 minutes', 'buy', '0x6666777788889999aaaa0000bbbbccccddddeeee', 'BSHARK', 100000000000, 2300),
    ('0xwhale123456789abcdef123456789abcdef12345', '0xtx005', 1000005, NOW() - INTERVAL '3 hours', 'buy', '0xcccc1111dddd2222eeee3333ffff44445555aaaa', 'WLAMBO', 5000000, 28350),
    ('0xtrader987654321fedcba987654321fedcba9876', '0xtx006', 1000006, NOW() - INTERVAL '45 minutes', 'sell', '0x1111aaaa2222bbbb3333cccc4444dddd5555eeee', 'FGOLD', 15000000, 3510),
    ('0xdev111222333444555666777888999aaabbbccc', '0xtx007', 1000007, NOW() - INTERVAL '6 hours', 'sell', '0xfedcba9876543210fedcba9876543210fedcba98', 'RKTFI', 200000000, 2400),
    ('0xsniper444555666777888999aaabbbcccdddeeef', '0xtx008', 1000008, NOW() - INTERVAL '5 minutes', 'buy', '0x9876543210fedcba9876543210fedcba98765432', 'SGAINS', 8000000, 12480)
ON CONFLICT (tx_hash, wallet_address, token_address, action) DO NOTHING;

-- Seed Alert Events
INSERT INTO alert_events (alert_type, token_address, token_symbol, wallet_address, title, message, bee_score, amount_usd, change_percent, created_at)
VALUES
    ('filter_match', '0x1234567890abcdef1234567890abcdef12345678', 'PEPE', NULL, 'Safe New Launches Match', 'PEPE matched your filter with BeeScore 78', 78, NULL, NULL, NOW() - INTERVAL '15 minutes'),
    ('whale_buy', '0x1234567890abcdef1234567890abcdef12345678', 'PEPE', '0xwhale123456789abcdef123456789abcdef12345', 'Whale Alert', 'Smart Money Whale #1 bought $12,500 of PEPE', NULL, 12500, NULL, NOW() - INTERVAL '30 minutes'),
    ('price_pump', '0x5555666677778888999900001111222233334444', 'DAPE', NULL, 'Volume Spike', 'DAPE volume increased 340% in the last hour', NULL, NULL, 340, NOW() - INTERVAL '1 hour'),
    ('price_pump', '0xcccc1111dddd2222eeee3333ffff44445555aaaa', 'WLAMBO', NULL, 'Price Pump Alert', 'WLAMBO pumped 67.8% in 1h', NULL, NULL, 67.8, NOW() - INTERVAL '45 minutes'),
    ('new_token', '0x6666777788889999aaaa0000bbbbccccddddeeee', 'BSHARK', NULL, 'New Token: BSHARK', 'New token BabyShark launched on PancakeSwap', NULL, NULL, NULL, NOW() - INTERVAL '8 hours'),
    ('high_bee_score', '0x5555666677778888999900001111222233334444', 'DAPE', NULL, 'High BeeScore Alert', 'DAPE has achieved BeeScore of 92', 92, NULL, NULL, NOW() - INTERVAL '2 hours'),
    ('whale_sell', '0xfedcba9876543210fedcba9876543210fedcba98', 'RKTFI', '0xdev111222333444555666777888999aaabbbccc', 'Dev Wallet Activity', 'Known Dev sold $2,400 of RKTFI', NULL, 2400, NULL, NOW() - INTERVAL '6 hours')
ON CONFLICT DO NOTHING;

