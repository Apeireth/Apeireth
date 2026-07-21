"""V1013 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import pytest
from apeireth.v1013_multi_tenant import (
    V1013_VERSION, Tenant, RBACRole, User, V1013MultiTenant,
)


class TestV1013:
    def test_init(self):
        mt = V1013MultiTenant()
        assert mt.n_roles() == 3
        assert mt.n_tenants() == 0
        assert mt.n_users() == 0

    def test_register_tenant(self):
        mt = V1013MultiTenant()
        t = mt.register_tenant("TestCorp")
        assert t.name == "TestCorp"
        assert mt.n_tenants() == 1

    def test_register_role(self):
        mt = V1013MultiTenant()
        mt.register_role(RBACRole(role_id="custom", name="Custom", permissions={"x"}))
        assert mt.n_roles() == 4

    def test_add_user(self):
        mt = V1013MultiTenant()
        t = mt.register_tenant("TestCorp")
        u = mt.add_user(t.tenant_id, {"admin"})
        assert u.tenant_id == t.tenant_id
        assert "admin" in u.roles

    def test_add_user_unknown_tenant(self):
        mt = V1013MultiTenant()
        with pytest.raises(ValueError):
            mt.add_user("unknown")

    def test_check_permission_admin(self):
        """V1013 真测 admin 全部权限 (主 17:43 实事求是)."""
        mt = V1013MultiTenant()
        t = mt.register_tenant("TestCorp")
        u = mt.add_user(t.tenant_id, {"admin"})
        assert mt.check_permission(u.user_id, "write") is True
        assert mt.check_permission(u.user_id, "read") is True
        assert mt.check_permission(u.user_id, "any") is True

    def test_check_permission_reader(self):
        mt = V1013MultiTenant()
        t = mt.register_tenant("TestCorp")
        u = mt.add_user(t.tenant_id, {"reader"})
        assert mt.check_permission(u.user_id, "read") is True
        assert mt.check_permission(u.user_id, "write") is False

    def test_check_permission_writer(self):
        mt = V1013MultiTenant()
        t = mt.register_tenant("TestCorp")
        u = mt.add_user(t.tenant_id, {"writer"})
        assert mt.check_permission(u.user_id, "read") is True
        assert mt.check_permission(u.user_id, "write") is True

    def test_check_permission_unknown_user(self):
        mt = V1013MultiTenant()
        assert mt.check_permission("unknown", "read") is False

    def test_check_resource_quota(self):
        """V1013 真测 ResourceQuota 真借鉴 (主 19:33)."""
        mt = V1013MultiTenant()
        t = mt.register_tenant("TestCorp", quota={"cpu": 10.0})
        assert mt.check_resource_quota(t.tenant_id, "cpu", 5.0) is True
        assert mt.check_resource_quota(t.tenant_id, "cpu", 15.0) is False

    def test_check_resource_quota_disabled(self):
        mt = V1013MultiTenant()
        t = mt.register_tenant("TestCorp")
        t.enabled = False
        assert mt.check_resource_quota(t.tenant_id, "cpu", 1.0) is False

    def test_authenticate(self):
        mt = V1013MultiTenant()
        t = mt.register_tenant("TestCorp")
        result = mt.authenticate(t.api_key)
        assert result is not None
        assert result.tenant_id == t.tenant_id

    def test_authenticate_invalid(self):
        mt = V1013MultiTenant()
        mt.register_tenant("TestCorp")
        assert mt.authenticate("invalid") is None

    def test_stats(self):
        mt = V1013MultiTenant()
        s = mt.stats()
        assert s["n_roles"] == 3
        assert s["version"] == V1013_VERSION

    def test_v22_33_asi_integration(self):
        """V1013 真测主 22:33 ASI 北极星."""
        mt = V1013MultiTenant()
        s = mt.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_k8s_auth0(self):
        """V1013 真测主 19:33 K8s + Auth0 + NIST RBAC 真借鉴."""
        mt = V1013MultiTenant()
        # 3 默认 roles
        assert "admin" in mt.roles
        assert "reader" in mt.roles
        assert "writer" in mt.roles

    def test_v17_43_truth(self):
        """V1013 真测主 17:43 实事求是 (真权限检查, 不假装)."""
        mt = V1013MultiTenant()
        t = mt.register_tenant("TestCorp")
        u = mt.add_user(t.tenant_id, {"reader"})
        # reader 不能 write
        assert mt.check_permission(u.user_id, "write") is False

    def test_complete_integration(self):
        """V1013 真测完整 multi-tenant (主 23:44 + 主 22:33 + 主 19:33 + 主 17:43)."""
        mt = V1013MultiTenant()
        t1 = mt.register_tenant("Corp1")
        t2 = mt.register_tenant("Corp2")
        u1 = mt.add_user(t1.tenant_id, {"admin"})
        u2 = mt.add_user(t2.tenant_id, {"reader"})
        assert mt.n_tenants() == 2
        assert mt.n_users() == 2
        # admin 全权限, reader 只读
        assert mt.check_permission(u1.user_id, "any") is True
        assert mt.check_permission(u2.user_id, "write") is False