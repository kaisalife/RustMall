//! Proto 消息序列化/反序列化测试

use prost::Message;

#[test]
fn test_auth_register_request_serialization() {
    let request = proto::auth::RegisterRequest {
        email: "test@example.com".to_string(),
        password: "password123".to_string(),
        nickname: "TestUser".to_string(),
    };

    let mut buf = Vec::new();
    request.encode(&mut buf).unwrap();
    let decoded = proto::auth::RegisterRequest::decode(&buf[..]).unwrap();

    assert_eq!(decoded.email, "test@example.com");
    assert_eq!(decoded.password, "password123");
    assert_eq!(decoded.nickname, "TestUser");
}

#[test]
fn test_product_create_request_serialization() {
    let request = proto::product::CreateProductRequest {
        name: "Test Product".to_string(),
        description: "A test product".to_string(),
        price: 29.99,
        category_id: 1,
        stock: 100,
    };

    let mut buf = Vec::new();
    request.encode(&mut buf).unwrap();
    let decoded = proto::product::CreateProductRequest::decode(&buf[..]).unwrap();

    assert_eq!(decoded.name, "Test Product");
    assert_eq!(decoded.description, "A test product");
    assert!((decoded.price - 29.99).abs() < 0.001);
    assert_eq!(decoded.category_id, 1);
    assert_eq!(decoded.stock, 100);
}

#[test]
fn test_order_create_request_serialization() {
    let request = proto::order::CreateOrderRequest {
        user_id: 42,
        items: vec![
            proto::order::OrderItem {
                product_id: 1,
                quantity: 2,
                unit_price: 9.99,
            },
            proto::order::OrderItem {
                product_id: 2,
                quantity: 3,
                unit_price: 19.99,
            },
        ],
    };

    let mut buf = Vec::new();
    request.encode(&mut buf).unwrap();
    let decoded = proto::order::CreateOrderRequest::decode(&buf[..]).unwrap();

    assert_eq!(decoded.user_id, 42);
    assert_eq!(decoded.items.len(), 2);
    assert_eq!(decoded.items[0].product_id, 1);
    assert_eq!(decoded.items[0].quantity, 2);
    assert!((decoded.items[0].unit_price - 9.99).abs() < 0.001);
    assert_eq!(decoded.items[1].product_id, 2);
    assert_eq!(decoded.items[1].quantity, 3);
    assert!((decoded.items[1].unit_price - 19.99).abs() < 0.001);
}

#[test]
fn test_inventory_deduct_request_serialization() {
    let request = proto::inventory::DeductStockRequest {
        product_id: 99,
        quantity: 5,
    };

    let mut buf = Vec::new();
    request.encode(&mut buf).unwrap();
    let decoded = proto::inventory::DeductStockRequest::decode(&buf[..]).unwrap();

    assert_eq!(decoded.product_id, 99);
    assert_eq!(decoded.quantity, 5);
}
