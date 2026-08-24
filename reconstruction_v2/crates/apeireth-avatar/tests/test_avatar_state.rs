use apeireth_avatar::{ExpressionMapper, Live2dController, Live2dConfig, Live2dExpression, VrmController, VrmConfig};
use apeireth_companion::emotion::Pad;

#[test]
fn test_expression_mapper_joy_and_drowsy() {
    let joyful_pad = Pad { pleasure: 0.9, arousal: 0.8, dominance: 0.7 };
    let expr = ExpressionMapper::map_pad_to_live2d(&joyful_pad, 0.2);
    assert_eq!(expr, Live2dExpression::Joy);

    let vrm_bs = ExpressionMapper::map_pad_to_vrm(&joyful_pad, 0.2);
    assert!(vrm_bs.happy > 0.5);

    let drowsy_pad = Pad { pleasure: 0.5, arousal: 0.2, dominance: 0.5 };
    let drowsy_expr = ExpressionMapper::map_pad_to_live2d(&drowsy_pad, 0.75);
    assert_eq!(drowsy_expr, Live2dExpression::Drowsy);
}

#[test]
fn test_live2d_controller_state() {
    let mut ctrl = Live2dController::new(Live2dConfig::default());
    ctrl.set_expression(Live2dExpression::Joy);
    assert_eq!(ctrl.current_expression, Live2dExpression::Joy);
    assert_eq!(ctrl.params.mouth_form, 1.0);

    ctrl.update_lip_sync(0.8, 0.5);
    assert_eq!(ctrl.params.mouth_open_y, 0.8);
}

#[test]
fn test_vrm_controller_state() {
    let mut ctrl = VrmController::new(VrmConfig::default());
    ctrl.set_look_at(0.5, 1.6, -0.2);
    assert_eq!(ctrl.look_at.x, 0.5);
}
