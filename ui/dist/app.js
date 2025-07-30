// Agent HUD v5 - Frontend Application Logic
class AgentHUDApp {
    constructor() {
        this.agents = [];
        this.humanRequests = [];
        this.contentItems = []; // New: content emissions
        this.latestContent = null; // New: latest content item
        this.currentContentIndex = -1; // New: current content index for navigation
        this.currentRequestId = null;
        this.activeTab = 'all';
        this.activeMainTab = 'requests'; // New: main tab state
        this.ws = null;
        this.reconnectInterval = null;
        
        this.init();
    }
    
    async init() {
        console.log('Initializing Agent HUD v5...');
        console.log('Initializing WebSocket connection...');
        
        // Initialize WebSocket connection
        this.connectToWebSocket();
        
        this.setupEventListeners();
        this.setupClearButtons(); // Add clear button event listeners
        console.log('Agent HUD v5 initialization complete');
    }
    
    async connectToWebSocket() {
        console.log('🔌 Discovering WebSocket server...');
        
        // Try ports 8080-8200 to match the Rust server's port discovery
        for (let port = 8080; port < 8200; port++) {
            try {
                await this.tryConnectToPort(port);
                return; // Connection successful
            } catch (error) {
                console.log(`Port ${port} not available, trying next...`);
                continue;
            }
        }
        
        console.error('❌ Could not find WebSocket server on any port');
        document.getElementById('connectionStatus').textContent = 'Server Not Found';
    }
    
    tryConnectToPort(port) {
        return new Promise((resolve, reject) => {
            console.log(`🔌 Trying to connect to port ${port}...`);
            const ws = new WebSocket(`ws://127.0.0.1:${port}`);
            
            ws.onopen = () => {
                console.log(`✅ WebSocket connected on port ${port}`);
                this.ws = ws; // Store the successful connection
                document.getElementById('connectionStatus').textContent = 'Connected';
                
                // Clear any reconnection interval
                if (this.reconnectInterval) {
                    clearTimeout(this.reconnectInterval);
                    this.reconnectInterval = null;
                }
                
                // Register as GUI client
                this.ws.send(JSON.stringify({
                    type: 'register-gui'
                }));
                
                resolve(); // Resolve the promise on successful connection
            };
            
            ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    console.log('📨 Received:', data);
                    this.handleWebSocketMessage(data);
                } catch (e) {
                    console.error('❌ Failed to parse message:', e);
                }
            };
            
            ws.onclose = () => {
                console.log('❌ WebSocket disconnected');
                document.getElementById('connectionStatus').textContent = 'Disconnected';
                
                // Attempt to reconnect after 3 seconds
                this.reconnectInterval = setTimeout(() => this.connectToWebSocket(), 3000);
            };
            
            ws.onerror = (error) => {
                console.error(`❌ WebSocket error on port ${port}:`, error);
                reject(error); // Reject the promise on error
            };
            
            // Set a timeout for connection attempt
            setTimeout(() => {
                if (ws.readyState !== WebSocket.OPEN) {
                    ws.close();
                    reject(new Error(`Connection timeout on port ${port}`));
                }
            }, 1000); // 1 second timeout per port
        });
    }
    
    handleWebSocketMessage(data) {
        switch (data.type) {
            case 'agent-connected':
                this.addAgent(data.data);
                break;
            case 'agent-disconnected':
                this.removeAgent(data.data.agentId);
                break;
            case 'human-input-request':
                this.addRequest(data.data);
                break;
            case 'agent-update':
                console.log('Agent update:', data.data);
                break;
            case 'content-emission':
                this.addContentItem(data.data);
                break;
            case 'markdown-content':
                this.addContentItem({
                    type: 'markdown',
                    content: data.data.content,
                    title: data.data.title || 'Markdown Content',
                    agent_id: data.data.agent_id,
                    agent_name: data.data.agent_name,
                    timestamp: data.data.timestamp || new Date().toISOString()
                });
                break;
            case 'code-content':
                this.addContentItem({
                    type: 'code',
                    content: data.data.content,
                    language: data.data.language || 'text',
                    title: data.data.title || 'Code Snippet',
                    agent_id: data.data.agent_id,
                    agent_name: data.data.agent_name,
                    timestamp: data.data.timestamp || new Date().toISOString()
                });
                break;
            case 'image-content':
                this.addContentItem({
                    type: 'image',
                    content: data.data.url || data.data.content,
                    caption: data.data.caption,
                    title: data.data.title || 'Image',
                    agent_id: data.data.agent_id,
                    agent_name: data.data.agent_name,
                    timestamp: data.data.timestamp || new Date().toISOString()
                });
                break;
            default:
                console.log('Unknown message type:', data.type);
        }
    }
    
    addAgent(agent) {
        const existing = this.agents.find(a => a.id === agent.id);
        if (!existing) {
            this.agents.push(agent);
            this.updateUI();
        }
    }
    
    removeAgent(agentId) {
        this.agents = this.agents.filter(a => a.id !== agentId);
        this.updateUI();
    }
    
    addRequest(request) {
        const existing = this.humanRequests.find(r => r.id === request.id);
        if (!existing) {
            this.humanRequests.push(request);
            this.updateUI();
        }
    }
    
    addContentItem(contentItem) {
        // Add unique ID if not present
        if (!contentItem.id) {
            contentItem.id = Date.now() + '-' + Math.random().toString(36).substr(2, 9);
        }
        
        const existing = this.contentItems.find(c => c.id === contentItem.id);
        if (!existing) {
            this.contentItems.push(contentItem);
            this.latestContent = contentItem;
            this.currentContentIndex = this.contentItems.length - 1; // Set to latest
            this.updateUI();
            console.log('📄 New content item:', contentItem.type, contentItem.title);
        }
    }
    
    updateUI() {
        this.updateStats();
        this.updateAgentList();
        this.updateRequestsTable();
        this.updateLatestContent();
        this.updateContentHistory();
    }
    
    updateStats() {
        const activeAgents = this.agents.filter(a => a.status === 'Active' || a.status === 'Connected').length;
        const pendingRequests = this.humanRequests.filter(r => r.status === 'Pending').length;
        const criticalRequests = this.humanRequests.filter(r => 
            r.status === 'Pending' && r.priority === 'Critical'
        ).length;
        
        document.getElementById('activeAgentsCount').textContent = activeAgents;
        document.getElementById('pendingRequestsCount').textContent = pendingRequests;
        document.getElementById('criticalRequestsCount').textContent = criticalRequests;
        document.getElementById('totalContentCount').textContent = this.contentItems.length;
    }
    
    updateAgentList() {
        const agentList = document.getElementById('agentList');
        
        if (this.agents.length === 0) {
            agentList.innerHTML = `
                <li class="empty-state">
                    <div class="empty-state-icon">🤖</div>
                    <div>No agents connected</div>
                </li>
            `;
            return;
        }
        
        agentList.innerHTML = this.agents.map(agent => `
            <li class="agent-item">
                <div class="agent-avatar">
                    ${agent.name.charAt(0).toUpperCase()}
                </div>
                <div class="agent-info">
                    <div class="agent-name">${agent.name}</div>
                    <div class="agent-status">
                        <span class="status-indicator ${agent.status.toLowerCase()}">
                            <span class="status-dot"></span>
                            ${agent.status}
                        </span>
                    </div>
                </div>
            </li>
        `).join('');
    }
    
    updateRequestsTable() {
        const table = document.getElementById('requestsTable');
        const tbody = table.querySelector('tbody');
        
        let filteredRequests = this.humanRequests;
        
        // Apply tab filter
        if (this.activeTab === 'pending') {
            filteredRequests = this.humanRequests.filter(r => r.status === 'Pending');
        } else if (this.activeTab === 'completed') {
            filteredRequests = this.humanRequests.filter(r => r.status === 'Completed');
        }
        
        if (filteredRequests.length === 0) {
            tbody.innerHTML = `
                <tr>
                    <td colspan="7" class="empty-state">
                        <div class="empty-state-icon">📋</div>
                        <div>No ${this.activeTab} requests found</div>
                    </td>
                </tr>
            `;
            return;
        }
        
        tbody.innerHTML = filteredRequests.map(request => `
            <tr>
                <td>
                    <div class="agent-name">${request.agent_name}</div>
                    <div style="font-size: 11px; color: #6b7280;">${request.agent_id.substring(0, 8)}</div>
                </td>
                <td>
                    <span class="priority-badge ${request.request_type.toLowerCase()}">
                        ${request.request_type}
                    </span>
                </td>
                <td>
                    <div style="max-width: 300px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                        ${request.message}
                    </div>
                </td>
                <td>
                    <span class="priority-badge ${request.priority.toLowerCase()}">
                        ${request.priority}
                    </span>
                </td>
                <td>
                    <span class="status-badge ${request.status.toLowerCase()}">
                        ${request.status}
                    </span>
                </td>
                <td>
                    <div style="font-size: 12px; color: #6b7280;">
                        ${this.formatTimestamp(request.timestamp)}
                    </div>
                </td>
                <td>
                    ${request.status === 'Pending' ? `
                        <button class="btn primary" onclick="app.openResponseModal('${request.id}')">
                            Respond
                        </button>
                    ` : `
                        <button class="btn" onclick="app.viewRequest('${request.id}')">
                            View
                        </button>
                    `}
                </td>
            </tr>
        `).join('');
    }
    
    formatTimestamp(timestamp) {
        try {
            const date = new Date(timestamp);
            const now = new Date();
            const diffMs = now - date;
            const diffMins = Math.floor(diffMs / 60000);
            const diffHours = Math.floor(diffMins / 60);
            
            if (diffMins < 1) return 'Just now';
            if (diffMins < 60) return `${diffMins}m ago`;
            if (diffHours < 24) return `${diffHours}h ago`;
            return date.toLocaleDateString();
        } catch (error) {
            return 'Unknown';
        }
    }
    
    openResponseModal(requestId) {
        const request = this.humanRequests.find(r => r.id === requestId);
        if (!request) return;
        
        this.currentRequestId = requestId;
        
        // Populate modal
        document.getElementById('modalAgentName').textContent = request.agent_name;
        document.getElementById('modalRequestMessage').textContent = request.message;
        document.getElementById('responseInput').value = '';
        
        // Handle options
        const optionsContainer = document.getElementById('modalOptions');
        const optionsList = document.getElementById('optionsList');
        
        if (request.options && request.options.length > 0) {
            optionsContainer.style.display = 'block';
            optionsList.innerHTML = request.options.map(option => `
                <button class="option-button" onclick="app.selectOption('${option}')">
                    ${option}
                </button>
            `).join('');
        } else {
            optionsContainer.style.display = 'none';
        }
        
        // Show modal
        document.getElementById('responseModal').classList.add('show');
    }
    
    closeResponseModal() {
        document.getElementById('responseModal').classList.remove('show');
        this.currentRequestId = null;
    }
    
    selectOption(option) {
        document.getElementById('responseInput').value = option;
    }
    
    async submitResponse() {
        if (!this.currentRequestId) return;
        
        const response = document.getElementById('responseInput').value.trim();
        if (!response) {
            this.showError('Please enter a response');
            return;
        }
        
        try {
            if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                this.ws.send(JSON.stringify({
                    type: 'human-input-response',
                    requestId: this.currentRequestId,
                    response: response
                }));
                
                // Update local state
                const request = this.humanRequests.find(r => r.id === this.currentRequestId);
                if (request) {
                    request.status = 'Completed';
                }
                
                this.closeResponseModal();
                this.updateUI();
                this.showSuccess('Response sent successfully');
            } else {
                this.showError('WebSocket connection not available');
            }
            
        } catch (error) {
            console.error('Failed to send response:', error);
            this.showError('Failed to send response');
        }
    }
    
    viewRequest(requestId) {
        // For now, just open the response modal in view mode
        this.openResponseModal(requestId);
        
        // Disable inputs for completed requests
        const request = this.humanRequests.find(r => r.id === requestId);
        if (request && request.status !== 'Pending') {
            document.getElementById('responseInput').disabled = true;
            document.querySelectorAll('.option-button').forEach(btn => btn.disabled = true);
        }
    }
    
    setupEventListeners() {
        // Close modal when clicking outside
        document.getElementById('responseModal').addEventListener('click', (e) => {
            if (e.target.id === 'responseModal') {
                this.closeResponseModal();
            }
        });
        
        // Handle keyboard shortcuts
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                this.closeResponseModal();
            }
        });
    }
    
    setupClearButtons() {
        console.log('🔧 Setting up clear button event listeners');
        
        // Wait for DOM to be ready
        setTimeout(() => {
            // Find clear buttons by their onclick attributes
            const clearHistoryBtn = document.querySelector('button[onclick="clearContentHistory()"]');
            const clearRequestsBtn = document.querySelector('button[onclick="clearHumanRequests()"]');
            
            if (clearHistoryBtn) {
                console.log('✅ Found clear history button, adding event listener');
                clearHistoryBtn.addEventListener('click', (e) => {
                    e.preventDefault();
                    console.log('🔥 Clear history button clicked via addEventListener');
                    this.clearContentHistory();
                });
            } else {
                console.warn('⚠️ Clear history button not found');
            }
            
            if (clearRequestsBtn) {
                console.log('✅ Found clear requests button, adding event listener');
                clearRequestsBtn.addEventListener('click', (e) => {
                    e.preventDefault();
                    console.log('🔥 Clear requests button clicked via addEventListener');
                    this.clearHumanRequests();
                });
            } else {
                console.warn('⚠️ Clear requests button not found');
            }
        }, 1000); // Wait 1 second for DOM to be fully ready
    }
    
    showError(message) {
        // Simple error display - in a real app you'd want a proper notification system
        console.error(message);
        alert(`Error: ${message}`);
    }
    
    showSuccess(message) {
        // Simple success display
        console.log(message);
        // You could implement a toast notification here
    }
    
    updateLatestContent() {
        const titleEl = document.getElementById('latestContentTitle');
        const metaEl = document.getElementById('latestContentMeta');
        const bodyEl = document.getElementById('latestContentBody');
        const counterEl = document.getElementById('contentCounter');
        const prevBtn = document.getElementById('prevContentBtn');
        const nextBtn = document.getElementById('nextContentBtn');
        
        if (!this.latestContent || this.contentItems.length === 0) {
            titleEl.textContent = 'Latest Content';
            metaEl.textContent = 'No content yet';
            counterEl.textContent = '0 / 0';
            prevBtn.disabled = true;
            nextBtn.disabled = true;
            bodyEl.innerHTML = `
                <div class="empty-content">
                    <div class="empty-content-icon">📝</div>
                    <div>No content emitted yet</div>
                    <div style="font-size: 14px; margin-top: 8px;">Agents can emit markdown, code, or images</div>
                </div>
            `;
            return;
        }
        
        const content = this.latestContent;
        titleEl.textContent = content.title;
        metaEl.textContent = `${content.type.toUpperCase()} • ${content.agent_name} • ${this.formatTimestamp(content.timestamp)}`;
        
        // Update navigation
        counterEl.textContent = `${this.currentContentIndex + 1} / ${this.contentItems.length}`;
        prevBtn.disabled = false;
        nextBtn.disabled = false;
        
        bodyEl.innerHTML = this.renderContent(content);
        
        // Trigger syntax highlighting after rendering
        setTimeout(() => {
            if (typeof Prism !== 'undefined') {
                Prism.highlightAll();
            }
        }, 100);
    }
    
    updateContentHistory() {
        const historyEl = document.getElementById('contentHistory');
        
        if (this.contentItems.length === 0) {
            historyEl.innerHTML = `
                <div class="empty-content">
                    <div class="empty-content-icon">📚</div>
                    <div>No content history</div>
                    <div style="font-size: 14px; margin-top: 8px;">All emitted content will appear here chronologically</div>
                </div>
            `;
            return;
        }
        
        // Sort by timestamp (most recent first)
        const sortedContent = [...this.contentItems].sort((a, b) => 
            new Date(b.timestamp) - new Date(a.timestamp)
        );
        
        historyEl.innerHTML = sortedContent.map(item => {
            const isActive = this.latestContent && this.latestContent.id === item.id;
            return `
                <div class="content-item ${isActive ? 'active' : ''}" onclick="app.viewContentItem('${item.id}')">
                    <div class="content-item-header">
                        <div>
                            <span class="content-type-badge ${item.type}">${item.type}</span>
                            <span style="margin-left: 8px; font-weight: 500;">${item.title}</span>
                        </div>
                        <div class="content-timestamp">${this.formatTimestamp(item.timestamp)}</div>
                    </div>
                    <div class="content-preview">
                        ${this.generateContentPreview(item)}
                    </div>
                </div>
            `;
        }).join('');
    }
    
    renderContent(content) {
        switch (content.type) {
            case 'markdown':
                return `<div class="markdown-content">${marked.parse(content.content)}</div>`;
            
            case 'code':
                const escapedCode = this.escapeHtml(content.content);
                return `
                    <div class="code-content">
                        <div class="code-header">
                            <span class="code-language">${content.language}</span>
                            <button class="code-copy-btn" onclick="app.copyCode('${content.id}')">Copy</button>
                        </div>
                        <div class="code-body">
                            <pre id="code-${content.id}"><code class="language-${content.language}">${escapedCode}</code></pre>
                        </div>
                    </div>
                `;
            
            case 'image':
                return `
                    <div class="image-content">
                        <img src="${content.content}" alt="${content.title}" loading="lazy">
                        ${content.caption ? `<div class="image-caption">${content.caption}</div>` : ''}
                    </div>
                `;
            
            default:
                return `<div>Unsupported content type: ${content.type}</div>`;
        }
    }
    
    generateContentPreview(item) {
        switch (item.type) {
            case 'markdown':
                const strippedMarkdown = item.content.replace(/[#*`_~]/g, '').substring(0, 100);
                return strippedMarkdown + (item.content.length > 100 ? '...' : '');
            
            case 'code':
                const firstLine = item.content.split('\n')[0].substring(0, 80);
                return firstLine + (item.content.length > 80 ? '...' : '');
            
            case 'image':
                return item.caption || 'Image content';
            
            default:
                return 'Content preview unavailable';
        }
    }
    
    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }
    
    copyCode(contentId) {
        const codeEl = document.getElementById(`code-${contentId}`);
        if (codeEl) {
            navigator.clipboard.writeText(codeEl.textContent).then(() => {
                this.showSuccess('Code copied to clipboard');
            }).catch(err => {
                console.error('Failed to copy code:', err);
                this.showError('Failed to copy code');
            });
        }
    }
    
    viewContentItem(contentId) {
        const contentIndex = this.contentItems.findIndex(c => c.id === contentId);
        if (contentIndex !== -1) {
            this.currentContentIndex = contentIndex;
            this.latestContent = this.contentItems[contentIndex];
            
            // Switch to latest tab first
            this.activeMainTab = 'latest';
            
            // Update tab buttons manually
            document.querySelectorAll('.main-tab').forEach(tab => tab.classList.remove('active'));
            document.querySelectorAll('.tab-pane').forEach(pane => pane.classList.remove('active'));
            
            // Find and activate the latest tab
            const latestTabBtn = document.querySelector('.main-tab[onclick*="latest"]');
            const latestTabPane = document.getElementById('latestTab');
            
            if (latestTabBtn) latestTabBtn.classList.add('active');
            if (latestTabPane) latestTabPane.classList.add('active');
            
            // Force update the content
            this.updateLatestContent();
            this.updateContentHistory(); // Refresh history to show active state
            
            console.log('📄 Viewing content:', this.latestContent.type, this.latestContent.title);
        }
    }
    
    navigateContentPrevious() {
        if (this.contentItems.length === 0) return;
        
        if (this.currentContentIndex > 0) {
            this.currentContentIndex--;
        } else {
            this.currentContentIndex = this.contentItems.length - 1; // Wrap to last
        }
        
        this.latestContent = this.contentItems[this.currentContentIndex];
        this.updateLatestContent();
    }
    
    navigateContentNext() {
        if (this.contentItems.length === 0) return;
        
        if (this.currentContentIndex < this.contentItems.length - 1) {
            this.currentContentIndex++;
        } else {
            this.currentContentIndex = 0; // Wrap to first
        }
        
        this.latestContent = this.contentItems[this.currentContentIndex];
        this.updateLatestContent();
    }
    
    clearContentHistory() {
        console.log('🧹 AgentHUDApp.clearContentHistory called');
        if (confirm('Are you sure you want to clear all content history?')) {
            this.contentItems = [];
            this.latestContent = null;
            this.currentContentIndex = -1; // Reset the content index
            this.updateUI();
            this.showSuccess('Content history cleared');
            console.log('📚 Content history cleared, items:', this.contentItems.length);
        } else {
            console.log('📚 Content history clear cancelled by user');
        }
    }
    
    clearHumanRequests() {
        console.log('🧹 AgentHUDApp.clearHumanRequests called');
        if (confirm('Are you sure you want to clear all human input requests?')) {
            this.humanRequests = [];
            this.currentRequestId = null; // Reset current request
            this.updateUI();
            this.showSuccess('Human requests cleared');
            console.log('🗑️ Human requests cleared, items:', this.humanRequests.length);
        } else {
            console.log('🗑️ Human requests clear cancelled by user');
        }
    }
    
    switchMainTab(tabName) {
        this.activeMainTab = tabName;
        
        // Update UI components
        this.updateUI();
        
        // Trigger syntax highlighting for code content
        if (tabName === 'latest' || tabName === 'history') {
            setTimeout(() => {
                if (typeof Prism !== 'undefined') {
                    Prism.highlightAll();
                }
            }, 100);
        }
    }
    
    // WebSocket connection handles all data loading
    // No need for mock data
}

// Global functions for HTML onclick handlers
function switchMainTab(tabName) {
    // Update main tab buttons
    document.querySelectorAll('.main-tab').forEach(tab => tab.classList.remove('active'));
    event.target.classList.add('active');
    
    // Update tab panes
    document.querySelectorAll('.tab-pane').forEach(pane => pane.classList.remove('active'));
    document.getElementById(tabName + 'Tab').classList.add('active');
    
    // Update app state
    app.activeMainTab = tabName;
    
    // Trigger syntax highlighting for code content if switching to latest/history
    if (tabName === 'latest' || tabName === 'history') {
        setTimeout(() => {
            if (typeof Prism !== 'undefined') {
                Prism.highlightAll();
            }
        }, 100);
    }
}

function switchTab(tabName) {
    // Update tab buttons
    document.querySelectorAll('.tab').forEach(tab => tab.classList.remove('active'));
    event.target.classList.add('active');
    
    // Update app state and refresh table
    app.activeTab = tabName;
    app.updateRequestsTable();
}

function refreshData() {
    // Force reconnect to WebSocket to refresh data
    if (app.ws) {
        app.ws.close();
    }
    setTimeout(() => app.connectToWebSocket(), 100);
}

function closeResponseModal() {
    app.closeResponseModal();
}

function submitResponse() {
    app.submitResponse();
}

function clearContentHistory() {
    console.log('🔥 clearContentHistory function called');
    if (app && app.clearContentHistory) {
        app.clearContentHistory();
    } else {
        console.error('❌ app.clearContentHistory not available');
    }
}

function clearHumanRequests() {
    console.log('🔥 clearHumanRequests function called');
    if (app && app.clearHumanRequests) {
        app.clearHumanRequests();
    } else {
        console.error('❌ app.clearHumanRequests not available');
    }
}

// Collapsible functionality
function toggleStatsSection() {
    const content = document.getElementById('statsContent');
    const icon = document.getElementById('statsCollapseIcon');
    
    if (content && icon) {
        const isCollapsed = content.classList.contains('collapsed');
        
        if (isCollapsed) {
            content.classList.remove('collapsed');
            icon.classList.remove('collapsed');
            icon.textContent = '▼';
        } else {
            content.classList.add('collapsed');
            icon.classList.add('collapsed');
            icon.textContent = '▶';
        }
    }
}

function toggleSidebar() {
    const sidebar = document.getElementById('sidebar');
    const toggleBtn = document.getElementById('sidebarToggle');
    
    if (sidebar && toggleBtn) {
        const isCollapsed = sidebar.classList.contains('collapsed');
        
        if (isCollapsed) {
            sidebar.classList.remove('collapsed');
            toggleBtn.querySelector('span').textContent = '←';
        } else {
            sidebar.classList.add('collapsed');
            toggleBtn.querySelector('span').textContent = '→';
        }
        
        // Force a reflow to prevent layout issues
        setTimeout(() => {
            window.dispatchEvent(new Event('resize'));
        }, 300);
    }
}

// Initialize the application
const app = new AgentHUDApp();

// Make functions globally available as a backup
window.clearContentHistory = clearContentHistory;
window.clearHumanRequests = clearHumanRequests;
window.app = app;

console.log('🚀 Agent HUD v5 initialized with global functions:', {
    clearContentHistory: typeof window.clearContentHistory,
    clearHumanRequests: typeof window.clearHumanRequests,
    app: typeof window.app
});